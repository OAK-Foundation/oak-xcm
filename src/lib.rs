#![cfg_attr(not(feature = "std"), no_std)]

/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
use cumulus_primitives_core::ParaId;
use frame_support::pallet_prelude::*;
use sp_runtime::traits::Convert;
use sp_std::prelude::*;
use xcm::latest::prelude::*;
use xcm_executor::traits::WeightBounds;
mod xcm_config;

// NOTE: This is maximum cost in weight to schedule a 24-execution-time automation task on Turing currently
pub const MAX_XCM_TRANSACT_WEIGHT: u64 = 6_000_000_000;
pub const TURING_PARA_ID: u32 = 2114;

pub trait XcmInstructionGenerator<T: frame_system::Config> {
    fn create_schedule_xcmp_instruction(
        provided_id: Vec<u8>,
        execution_times: Vec<u64>,
        para_id: ParaId,
        returnable_call: Vec<u8>,
    ) -> xcm::v2::Instruction<()>;

    fn create_xcm_instruction_set(
        asset: MultiAsset,
        transact_instruction: xcm::v2::Instruction<()>,
        refund_account: T::AccountId,
    ) -> xcm::v2::Xcm<()>;
}

pub struct OakXcmInstructionGenerator<A, W>(PhantomData<(A, W)>);

impl<T, A, W> XcmInstructionGenerator<T> for OakXcmInstructionGenerator<A, W>
where
    T: frame_system::Config,
    A: Convert<T::AccountId, [u8; 32]>,
    W: WeightBounds<<T as frame_system::Config>::Call>,
{
    fn create_schedule_xcmp_instruction(
        provided_id: Vec<u8>,
        execution_times: Vec<u64>,
        para_id: ParaId,
        returnable_call: Vec<u8>,
    ) -> xcm::v2::Instruction<()> {
        let call = xcm_config::OakChainCallBuilder::automation_time_schedule_xcmp::<T>(
            provided_id,
            execution_times,
            para_id,
            returnable_call,
            MAX_XCM_TRANSACT_WEIGHT,
        );

        Transact::<()> {
            origin_type: OriginKind::Native,
            require_weight_at_most: MAX_XCM_TRANSACT_WEIGHT,
            call: call.encode().into(),
        }
    }

    // Generic Instruction Creation
    fn create_xcm_instruction_set(
        asset: MultiAsset,
        transact_instruction: xcm::v2::Instruction<()>,
        refund_account: T::AccountId,
    ) -> xcm::v2::Xcm<()> {
        let withdraw_asset_instruction = WithdrawAsset::<()>(vec![asset.clone()].into());
        let buy_execution_weight_instruction = BuyExecution::<()> {
            fees: asset.clone(),
            weight_limit: Unlimited,
        };
        let refund_surplus_instruction = RefundSurplus::<()>;
        let deposit_asset_instruction = DepositAsset::<()> {
            assets: MultiAssetFilter::Wild(All),
            max_assets: 1,
            beneficiary: MultiLocation {
                parents: 0,
                interior: X1(AccountId32 {
                    network: Any,
                    id: A::convert(refund_account),
                }),
            },
        };

        let execution_weight = W::weight(
            &mut Xcm(vec![
                withdraw_asset_instruction.clone(),
                buy_execution_weight_instruction,
                transact_instruction.clone(),
                refund_surplus_instruction.clone(),
                deposit_asset_instruction.clone(),
            ])
            .into(),
        )
        .unwrap();

        Xcm(vec![
            withdraw_asset_instruction,
            BuyExecution::<()> {
                fees: asset,
                weight_limit: Limited(execution_weight),
            },
            transact_instruction,
            refund_surplus_instruction,
            deposit_asset_instruction,
        ])
    }
}
