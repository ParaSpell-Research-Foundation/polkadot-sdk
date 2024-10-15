//! # xcNFT Pallet by ParaSpell✨ Foundation team
//! 
//! Pallet is made under [MIT](https://github.com/paraspell-research-foundation/xcNFT-Pallet/blob/main/LICENSE) license and is thereby free to use, modify, and distribute.
//!
//! A pallet that allows you to share your **Uniques** or **NFTs** across parachains.
//!
//!  As mentioned above, this pallet supports both Uniques and NFTs pallets for NFT management.
//!
//! ## Overview
//!
//! This pallet consists of following functionalities:
//! - Transferring empty collection cross-chain: **collectionXtransfer**  
//! - Transferring non-empty collection cross-chain (All NFTs are owned by collection owner): **collectionXtransfer**  
//! - Transfering non-empty collection cross-chain (NFTs are distributed between different accounts): **collectionXtransfer** & **collectionXtransferVote** & **collectionXtransferInitiate**
//! - Transfering non-fungible assets cross-chain: **nftXtransfer** & **nftXclaim**
//! - Updating collection metadata cross-chain: **collectionXupdate**
//! - Updating non-fungible asset metadata cross-chain: **nftXupdate**
//! - Burning collection cross-chain: **collectionXburn**
//! - Burning non-fungible asset cross-chain: **nftXburn**
//! - Transferring collection ownership cross-chain: **collectionXownership**
//! - Transferring non-fungible asset ownership cross-chain: **nftXownership**
//!
//! Each function within pallet has its own weight and is defined in `weights.rs` file.
//! 
//! Each function is also annotated with comments explaining the purpose and functionality design of the function.
//!
//! ## Dependencies
//! This pallet depends on the following pallets:
//! 
//! Substrate:
//! - `frame-benchmarking`
//! - `frame-support`
//! - `frame-system`
//! 
//! Cumulus:
//! - `cumulus-primitives-core`
//! - `cumulus-pallet-xcm`
//! 
//! XCMP:
//! - `xcm`
//! 
//! SP:
//! - `sp-runtime`
//! - `sp-std`
//! - `sp-core`
//! - `sp-io`
//! 
//! Substrate Pallets:
//! - `pallet-nfts`
//! - `pallet-uniques`
//! - `pallet-balances`
//! - `parachain-info`

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking; 



#[frame_support::pallet]
pub mod pallet {

	use cumulus_pallet_xcm::Origin;
use frame_system::pallet_prelude::*;
	use sp_runtime::{traits::{AccountIdLookup, CheckedAdd, One}, DispatchError};
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*, sp_runtime::traits::Hash, traits::{fungibles::metadata, Currency, LockableCurrency, ReservableCurrency}, DefaultNoBound};
	use frame_support::traits::PalletInfo;
	use codec::Encode;
	use cumulus_primitives_core::{ParaId};
	use scale_info::prelude::vec;
	use sp_std::prelude::*;
	use xcm::latest::prelude::*;
	use pallet_nfts::{AttributeNamespace, Call as NftsCall, ItemDetails, ItemMetadata, CollectionDetails, DestroyWitness};
	use core::marker::PhantomData;
	use crate::pallet::Call as XcNftCall;
	use log::{debug, info, trace};
 	use sp_runtime::DispatchErrorWithPostInfo;
	use frame_support::dispatch::PostDispatchInfo;
	use frame_system::{self as system, RawOrigin};
	use frame_support::traits::Incrementable;

	pub type BalanceOf<T, I = ()> =
	<<T as pallet_nfts::Config<I>>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
	use sp_runtime::traits::StaticLookup;
	type AccountIdLookupOf<T> = <<T as frame_system::Config>::Lookup as StaticLookup>::Source;

	pub type CollectionConfigFor<T, I = ()> = pallet_nfts::CollectionConfig<
	BalanceOf<T, I>, BlockNumberFor<T>,
	<T as pallet_nfts::Config<I>>::CollectionId,
	>;


	pub type ParachainID = ParaId; 

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config<I: 'static = ()>: frame_system::Config + pallet_nfts::Config<I> + parachain_info::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self, I>>
            + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The overarching call type; we assume sibling chains use the same type.
		type RuntimeCall:From<pallet_nfts::Call<Self, I>> + Encode; //Potom zmenit nazad na  From<Call<Self, I>> + Encode;

		///Runtime call for XcNftCall
		type XcNftCall: From<XcNftCall<Self, I>> + Encode;

		/// The sender to use for cross-chain messages.
		type XcmSender: SendXcm;

		/// A type representing the weights required by the dispatchables of this pallet.
		type WeightInfo: crate::weights::WeightInfo;

		/// Specifies how long should cross-chain proposals last
		type ProposalTimeInBlocks: Get<u32>;

		/// Specifies how much different owners can be in a collection - used in voting process
		type MaxOwners: Get<u32>;

	}

	#[pallet::pallet]
	pub struct Pallet<T, I = ()>(PhantomData<(T, I)>);

	#[derive(
		Encode, Decode, MaxEncodedLen, TypeInfo, Clone, PartialEq, Default, Debug
	)]
	#[scale_info(skip_type_params(T))]
	pub enum Vote {
		#[default]
		Aye,
		Nay
	}

	#[derive(
		Encode, Decode, MaxEncodedLen, TypeInfo, Clone, PartialEq, Default, Debug
	)]
	#[scale_info(skip_type_params(T,I))]
	pub struct Votes<T: Config<I>, I: 'static = ()> {
		aye: BoundedVec<T::AccountId, T::MaxOwners>,
		nay: BoundedVec<T::AccountId, T::MaxOwners>,
	}

	#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Clone, PartialEq, Default, Debug)]
    #[scale_info(skip_type_params(T, I))]
	pub struct Proposal<T: Config<I>, I: 'static = ()> {
		proposal_id: u64,
		collection_id: T::CollectionId,
		proposed_collection_owner: T::AccountId,
		proposed_destination_para: ParachainID,
		proposed_destination_config: CollectionConfigFor<T, I>,
		owners: BoundedVec<T::AccountId, T::MaxOwners>,
		number_of_votes: Votes<T,I>,
		end_time: BlockNumberFor<T>,
	}

	#[derive(
		Encode, Decode, MaxEncodedLen, TypeInfo, Clone, PartialEq, Default,
	)]
	#[scale_info(skip_type_params(T, I))]
	pub struct SentStruct<T: Config<I>, I: 'static = ()> {
		origin_para_id: ParachainID,
		origin_collection_id: T::CollectionId,
		origin_asset_id: T::ItemId,
		destination_collection_id: T::CollectionId,
		destination_asset_id: T::ItemId,
	}

	#[derive(
		Encode, Decode, MaxEncodedLen, TypeInfo, Clone, PartialEq, Default,
	)]
	#[scale_info(skip_type_params(T, I))]
	pub struct ReceivedStruct<T: Config<I>, I: 'static = ()> {
		origin_para_id: ParachainID,
		origin_collection_id: T::CollectionId,
		origin_asset_id: T::ItemId,
		received_collection_id: T::CollectionId,
		received_asset_id: T::ItemId,
	}

	#[derive(
		Encode, Decode, MaxEncodedLen, TypeInfo, Clone, PartialEq, Default,
	)]
	#[scale_info(skip_type_params(T, I))]
	pub struct ReceivedCols<T: Config<I>, I: 'static = ()> {
		origin_para_id: ParachainID,
		origin_collection_id: T::CollectionId,
		received_collection_id: T::CollectionId,
	}

	#[pallet::storage]
	#[pallet::getter(fn sent_assets)]
	pub type SentAssets<T: Config<I>, I: 'static = ()> = StorageMap<
    _,
    Blake2_128Concat,
    (T::CollectionId, T::ItemId),  // Tuple (origin_collection_id, origin_asset_id)
    SentStruct<T, I>,       // Sent assets structure
	>;

	#[pallet::storage]
	#[pallet::getter(fn received_assets)]
	pub type ReceivedAssets<T: Config<I>, I: 'static = ()> = StorageMap<
    _,
    Blake2_128Concat,
    (T::CollectionId, T::ItemId),  // Tuple (received_collection_id, received_asset_id)
    ReceivedStruct<T, I>,   // Received assets structure
	>;

	#[pallet::storage]
	#[pallet::getter(fn received_collections)]
	pub type ReceivedCollections<T: Config<I>, I: 'static = ()> = StorageMap<
    _,
    Blake2_128Concat,
    T::CollectionId, 
    ReceivedCols<T, I>,   // Received assets structure
	>;

	//Next proposal id
	#[pallet::storage]
	#[pallet::getter(fn next_proposal_id)]
	pub type NextProposalId<T: Config<I>, I: 'static = ()> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn cross_chain_proposals)]
	pub type CrossChainProposals<T: Config<I>, I: 'static = ()> = StorageMap<
	_,
	Blake2_128Concat,
	u64, // Proposal ID
	Proposal<T, I>,  // Proposal structure
	>;

	/// <https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html#event-and-error>
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]

	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// We usually use passive tense for events.
		SomethingStored { block_number: BlockNumberFor<T>, who: T::AccountId },

		/// Event emitted when a collection is transferred cross-chain
		CollectionTransferred {
			origin_collection_id: T::CollectionId,
			destination_para_id: ParachainID,
			to_address: AccountIdLookupOf<T>,
		},

		/// Event emitted when cross-chain transfer fails
		CollectionFailedToXCM {
			e: SendError,
			collection_id: T::CollectionId,
			owner: T::AccountId,
			destination: ParaId,
		},

		/// Event emitted when there are different NFT owners in the collection
		CollectionTransferProposalCreated {
			proposal_id: u64,
			collection_id: T::CollectionId,
			proposer: T::AccountId,
			proposed_collection_owner: AccountIdLookupOf<T>,
			destination: ParaId,
		},

		/// Event emitted when a collection and its NFTs are transferred cross-chain
		CollectionAndNFTsTransferred {
			origin_collection_id: T::CollectionId,
			nft_ids: Vec<T::ItemId>,
			destination_para_id: ParachainID,
			to_address: AccountIdLookupOf<T>,
		},

		/// Event emitted when a vote is registered
		CrossChainPropoposalVoteRegistered{
			proposal_id: u64,
			voter: T::AccountId,
			vote: Vote,
		},

		/// Event emitted when non-fungible asset is transferred cross-chain
		NFTTransferred {
			origin_collection_id: T::CollectionId,
			origin_asset_id: T::ItemId,
			destination_para_id: ParachainID,
			destination_collection_id: T::CollectionId,
			destination_asset_id: T::ItemId,
			to_address: AccountIdLookupOf<T>,
		},

		/// Event emitted when non-fungible asset is claimed into collection that was also sent cross-chain
		NFTClaimed {
			collection_claimed_from: T::CollectionId,
			asset_removed: T::ItemId,
			collection_claimed_to: T::CollectionId,
			asset_claimed: T::ItemId,
		},

		/// Event emitted when cross-chain collection metadata transfer fails
		ColMetadataFailedToXCM {
			e: SendError,
			collection_id: T::CollectionId,
			proposed_data: BoundedVec<u8, T::StringLimit>,
			owner: T::AccountId,
			destination: ParaId,
		},
	
		/// Event emitted when collection metadata is transferred cross-chain
		ColMetadataSent{
			collection_id: T::CollectionId,
			proposed_data: BoundedVec<u8, T::StringLimit>,
			owner: T::AccountId,
			destination: ParaId,
		},

		/// Event emitted when cross-chain NFT metadata transfer fails
		NFTMetadataFailedToXCM {
			e: SendError,
			collection_id: T::CollectionId,
			asset_id: T::ItemId,
			proposed_data: BoundedVec<u8, T::StringLimit>,
			owner: T::AccountId,
			destination: ParaId,
		},
			
		/// Event emitted when NFT metadata is transferred cross-chain
		NFTMetadataSent{
			collection_id: T::CollectionId,
			asset_id: T::ItemId,
			proposed_data: BoundedVec<u8, T::StringLimit>,
			owner: T::AccountId,
			destination: ParaId,
		},

		/// Event emitted when cross-chain collection burn call transfer fails
		ColBurnFailedToXCM {
			e: SendError,
			collection_id: T::CollectionId,
			burn_data: pallet_nfts::DestroyWitness,
			owner: T::AccountId,
			destination: ParaId,
		},
					
		/// Event emitted when collection burn call is transferred cross-chain
		ColBurnSent{
			collection_id: T::CollectionId,
			burn_data: pallet_nfts::DestroyWitness,
			owner: T::AccountId,
			destination: ParaId,
		},

		/// Event emitted when cross-chain NFT burn call transfer fails
		NFTBurnFailedToXCM {
			e: SendError,
			collection_id: T::CollectionId,
			asset_id: T::ItemId,
			owner: T::AccountId,
			destination: ParaId,
		},
							
		/// Event emitted when NFT burn call is transferred cross-chain
		NFTBurnSent{
			collection_id: T::CollectionId,
			asset_id: T::ItemId,
			owner: T::AccountId,
			destination: ParaId,
		},

		/// Event emitted when cross-chain Collection ownership call transfer fails
		ColOwnershipFailedToXCM {
			e: SendError,
			collection_id: T::CollectionId,
			proposed_owner: AccountIdLookupOf<T>,
			destination: ParaId,
		},
									
		/// Event emitted when Collection ownership call is transferred cross-chain
		ColOwnershipSent{
			collection_id: T::CollectionId,
			proposed_owner: AccountIdLookupOf<T>,
			destination: ParaId,
		},

		/// Event emitted when cross-chain NFT ownership call transfer fails
		NFTOwnershipFailedToXCM {
			e: SendError,
			collection_id: T::CollectionId,
			asset_id: T::ItemId,
			proposed_owner: AccountIdLookupOf<T>,
			destination: ParaId,
		},
											
		/// Event emitted when NFT ownership call is transferred cross-chain
		NFTOwnershipSent{
			collection_id: T::CollectionId,
			asset_id: T::ItemId,
			proposed_owner: AccountIdLookupOf<T>,
			destination: ParaId,
		},

		CollectionReceived {
			origin_collection_id: T::CollectionId,
			received_collection_id: T::CollectionId,
			to_address: AccountIdLookupOf<T>,
		},

		CollectionAlreadyReceived{
			origin_collection_id: T::CollectionId,
			to_address: AccountIdLookupOf<T>,
		},

		CollectionCreationFailed {
			error: DispatchError,
			owner: AccountIdLookupOf<T>,
		},

		CollectionBurnFailed {
			error: DispatchErrorWithPostInfo<PostDispatchInfo>,
			collection_id: T::CollectionId,
			owner: AccountIdLookupOf<T>,
		},


		CollectionMetadataSetFailed {
			error: DispatchError,
			collection_id: T::CollectionId,
			owner: AccountIdLookupOf<T>,
		},

		CollectionOwnershipTransferFailed {
			error: DispatchError,
			collection_id: T::CollectionId,
			owner: AccountIdLookupOf<T>,
		},

		NFTBurnFailed {
			error: DispatchError,
			collection_id: T::CollectionId,
			asset_id: T::ItemId,
			owner: AccountIdLookupOf<T>,
		},

		NFTMetadataSetFailed {
			error: DispatchError,
			collection_id: T::CollectionId,
			asset_id: T::ItemId,
			owner: AccountIdLookupOf<T>,
		},

		NFTOwnershipTransferFailed {
			error: DispatchError,
			collection_id: T::CollectionId,
			asset_id: T::ItemId,
			owner: AccountIdLookupOf<T>,
		},

		NFTMintFailed {
			error: DispatchError,
			collection_id: T::CollectionId,
			asset_id: T::ItemId,
			owner: AccountIdLookupOf<T>,
		},

		NFTReceived {
			origin_collection_id: T::CollectionId,
			origin_asset_id: T::ItemId,
			received_collection_id: T::CollectionId,
			received_asset_id: T::ItemId,
			to_address: AccountIdLookupOf<T>,
		},

		NFTReturnedToOrigin{
			origin_collection_id: T::CollectionId,
			origin_asset_id: T::ItemId,
			returned_from_collection_id: T::CollectionId,
			returned_from_asset_id: T::ItemId,
			to_address: T::AccountId
		},

		CollectionWithNftsReceived {
			collection_id: T::CollectionId,
			items: Vec<(T::ItemId, BoundedVec<u8, T::StringLimit>)>,
		},

	}

	/// <https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html#event-and-error>
	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Error names should be descriptive.
		CollectionDoesNotExist,
		NFTDoesNotExist,
		NFTExists,
		ProposalAlreadyExists,
		NotCollectionOwner,
		NFTAlreadyReceived,
		ProposalExpired,
		ProposalStillActive,
		ProposalDoesNotExist,
		ProposalDidNotPass,
		AlreadyVotedThis,
		MaxOwnersReached,
		NotNFTOwner,
		NFTNotReceived,
		AccountLookupFailed,
		NotSent,
		NoNextCollectionId,
		ConversionError,
		WrongOriginCollectionAtOrigin,
	}

	//#[pallet::hooks]
	//impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}


	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {

		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn collection_x_transfer(origin: OriginFor<T>, origin_collection: T::CollectionId, destination_para: ParaId, destination_account: AccountIdLookupOf<T>, config: CollectionConfigFor<T, I> ) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin.clone())?;

			//See if collection exists
			let collection_exists = pallet_nfts::Collection::<T, I>::contains_key(&origin_collection);
			ensure!(collection_exists, Error::<T, I>::CollectionDoesNotExist);

			//See if user owns the collection
			let owner = pallet_nfts::Pallet::<T,I>::collection_owner(origin_collection.clone()).ok_or(Error::<T, I>::CollectionDoesNotExist)?;
			ensure!(owner == who.clone(), Error::<T, I>::NotCollectionOwner);

			///Retrieve collection
			// Use the storage map directly
			let mut items = Vec::new();

			for (item_id, _item_details) in pallet_nfts::Item::<T, I>::iter_prefix(&origin_collection) {
				// Process or collect the item details
				// item_id is the ItemId, and item_details contains the item's details
				//Store items in an array
				items.push(item_id);
			}

			///Check if the collection is empty (items array is empty)
			if items.is_empty() {
				//Transfer the collection to the destination parachain
				match send_xcm::<T::XcmSender>(
					(Parent, Junction::Parachain(destination_para.into())).into(),
					Xcm(vec![
						UnpaidExecution { weight_limit: Unlimited, check_origin: None },
						Transact {
							origin_kind: OriginKind::SovereignAccount,
							require_weight_at_most: Weight::from_parts(1_000_000_000, 64 * 1024),
							call: <T as Config<I>>::XcNftCall::from(Call::<
								T,
								I,
							>::parse_collection_empty {
								origin_collection: origin_collection.clone(),
								receiving_account: destination_account.clone(),
								config,
								origin_para: parachain_info::Pallet::<T>::parachain_id(), 
							})
							.encode()
							.into(),
						},
					]),
				) {
					Ok((_hash, _cost)) => {
						//If collection was received, remove from received collections
						if ReceivedCollections::<T, I>::contains_key(&origin_collection) {
							ReceivedCollections::<T, I>::remove(&origin_collection);
						}

						//Burning the collection
						if let Some(col_deposit) = pallet_nfts::Pallet::<T, I>::deposit(origin_collection) {
							T::Currency::unreserve(&who.clone(), col_deposit);
						}

						pallet_nfts::Collection::<T, I>::remove(&origin_collection);

						pallet_nfts::CollectionMetadataOf::<T, I>::remove(&origin_collection);

						pallet_nfts::CollectionConfigOf::<T, I>::remove(&origin_collection);

						pallet_nfts::CollectionRoleOf::<T, I>::remove(&origin_collection, who.clone());

						pallet_nfts::CollectionAccount::<T, I>::remove(who.clone(), &origin_collection);
												
						// Emit an event.
						Self::deposit_event(Event::CollectionTransferred {
							origin_collection_id: origin_collection,
							destination_para_id: destination_para,
							to_address: destination_account,
						});
					},
					Err(e) => Self::deposit_event(Event::CollectionFailedToXCM {
						e,
						collection_id: origin_collection,
						owner: who.clone(),
						destination: destination_para,
					}),
				}
			}
			else {
				//Check if all the NFTs are owned by the collection owner
				let collection_owner = who.clone(); // The owner of the collection
				// Iterate through all items in the collection
				for item_id in items.clone() {
					// Retrieve the item details using the collection ID and item ID
					if let Some(nft_owner) = pallet_nfts::Pallet::<T, I>::owner(origin_collection, item_id) {
						// Check if the item owner is not the collection owner
						if nft_owner != collection_owner {
							//Create proposal

							for (_data, proposal) in CrossChainProposals::<T, I>::iter() {
								if proposal.collection_id == origin_collection {
									//Emit test event
									return Err(Error::<T, I>::ProposalAlreadyExists.into());
								}
							}

							//Find out how many different owners are there
							let mut different_owners = BoundedVec::new(); // Use Vec instead of a fixed-size array
							for item_id in items.clone() {
								if let Some(nft_owner) = pallet_nfts::Pallet::<T, I>::owner(origin_collection, item_id) {
									if nft_owner != collection_owner {
										//Check if owner is not present in different owners
										if !different_owners.contains(&nft_owner) {
											different_owners.try_push(nft_owner).ok(); 
										}
									}
								}
							}

							///+1 because the collection owner is not included in the different owners
							different_owners.try_push(collection_owner.clone()).ok();

							let proposal_id = NextProposalId::<T, I>::get();

							if proposal_id == 0 {
								NextProposalId::<T, I>::put(1);
							}
							else if proposal_id == u64::MAX {
								NextProposalId::<T, I>::put(1);
							}
							else {
								NextProposalId::<T, I>::put(proposal_id + 1);
							}

							let block_n: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();

							let proposal = Proposal::<T, I> {
								proposal_id: proposal_id,
								collection_id: origin_collection,
								proposed_collection_owner: T::Lookup::lookup(destination_account.clone())?, //Converting back to AccountId, if the proposal gets accepted, then converting it back to AccountIdLookup
								proposed_destination_config: config,
								proposed_destination_para: destination_para,
								owners: different_owners,
								number_of_votes: Votes {
									aye: BoundedVec::new(),
									nay: BoundedVec::new(),
								},
								end_time: block_n + T::ProposalTimeInBlocks::get().into(),
							};

							<CrossChainProposals<T, I>>::insert(proposal_id,proposal);

							Self::deposit_event(Event::CollectionTransferProposalCreated {
								proposal_id: proposal_id,
								collection_id: origin_collection,
								proposer: who.clone(),
								proposed_collection_owner: destination_account.clone(),
								destination: destination_para,							
							});

							return Ok(().into());
						} 
					}
				}
				///Transfer the collection to the destination parachain because owner of collection owns all NFTs
				//let collection_config = pallet_nfts::CollectionMetadataOf::<T, I>::get(&origin_collection).unwrap();
				//First check if collection contains any metadata
				let mut collection_metadata = None;
				if pallet_nfts::CollectionMetadataOf::<T, I>::contains_key(&origin_collection){
					collection_metadata = pallet_nfts::Pallet::<T, I>::collection_data(origin_collection);
				}

				if collection_metadata.is_none() {
					collection_metadata = Some(BoundedVec::new());
				}

				//Get NFT configs
				let mut nft_metadata = Vec::new();
				for item_id in items.clone() {
					if pallet_nfts::ItemMetadataOf::<T, I>::contains_key(&origin_collection, item_id) {
						let item_details = pallet_nfts::Pallet::<T, I>::item_data(origin_collection, item_id).unwrap();
						nft_metadata.push((item_id,item_details));
					}else{
						//Add empty metadata
						nft_metadata.push((item_id, BoundedVec::new()));
					}
				}

				//Send configs
				match send_xcm::<T::XcmSender>(
					(Parent, Junction::Parachain(destination_para.into())).into(),
					Xcm(vec![
						UnpaidExecution { weight_limit: Unlimited, check_origin: None },
						Transact {
							origin_kind: OriginKind::SovereignAccount,
							require_weight_at_most: Weight::from_parts(1_000_000_000, 64 * 1024),
							call: <T as Config<I>>::XcNftCall::from(Call::<
								T,
								I,
							>::parse_collection_same_owner {
								owner: destination_account.clone(),
								config,
								origin_collection_id: origin_collection.clone(),
								origin_para: parachain_info::Pallet::<T>::parachain_id(),
								collection_metadata: collection_metadata.unwrap(),
								nfts: nft_metadata,
							})
							.encode()
							.into(),
						},
					]),
				) {
					Ok((_hash, _cost)) => {
						//If collection was received, remove from received collections
						if ReceivedCollections::<T, I>::contains_key(&origin_collection) {
							ReceivedCollections::<T, I>::remove(&origin_collection);
						}

						//Burning the NFTs
						for item_id in items.clone() {
							let _ = pallet_nfts::Pallet::<T, I>::burn(origin.clone(), origin_collection.clone(), item_id);
						}
						//Burning the collection
						if let Some(col_deposit) = pallet_nfts::Pallet::<T, I>::deposit(origin_collection) {
							T::Currency::unreserve(&who.clone(), col_deposit);
						}

						pallet_nfts::Collection::<T, I>::remove(&origin_collection);

						pallet_nfts::CollectionMetadataOf::<T, I>::remove(&origin_collection);

						pallet_nfts::CollectionConfigOf::<T, I>::remove(&origin_collection);

						pallet_nfts::CollectionRoleOf::<T, I>::remove(&origin_collection, who.clone());

						pallet_nfts::CollectionAccount::<T, I>::remove(who.clone(), &origin_collection);

						// Emit an event
						Self::deposit_event(Event::CollectionAndNFTsTransferred {
							origin_collection_id: origin_collection,
							nft_ids: items,
							destination_para_id: destination_para,
							to_address: destination_account,
						});
					},
					Err(e) => Self::deposit_event(Event::CollectionFailedToXCM {
						e,
						collection_id: origin_collection,
						owner: who.clone(),
						destination: destination_para,
					}),
				}

			}
			Ok(().into())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn collection_x_transfer_vote(origin: OriginFor<T>, proposal_id: u64, actual_vote: Vote) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			let proposal = CrossChainProposals::<T, I>::get(proposal_id);
			ensure!(proposal.is_some(), Error::<T, I>::ProposalDoesNotExist); // Check before unwrapping
		
			// Safely unwrap the proposal once
			let mut unwrapped_proposal = proposal.unwrap();

			// See if the user can vote, check if they are in the owners list
			ensure!(unwrapped_proposal.owners.contains(&who), Error::<T, I>::NotNFTOwner);

		
			// Check if the proposal is still active
			let block_n: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
			if block_n > unwrapped_proposal.end_time {
				// If proposal did not pass (Less than 50% of votes are aye)
				// Remove proposal from proposals to free up storage space
				let number_of_votes = &unwrapped_proposal.number_of_votes.aye.len() + &unwrapped_proposal.number_of_votes.nay.len();
				if (unwrapped_proposal.number_of_votes.aye.len() < number_of_votes / 2 || unwrapped_proposal.number_of_votes.aye.len() == 0 && unwrapped_proposal.number_of_votes.nay.len() == 0) {
					return Err(Error::<T, I>::ProposalExpired.into());
				}
				return Err(Error::<T, I>::ProposalExpired.into());
			}

			//Check if the user has already voted, if they did, see if they voted the same or different. If same, return error and if different, update the vote
			if unwrapped_proposal.number_of_votes.aye.contains(&who) {
				if actual_vote == Vote::Nay {
					// Mutably borrow aye and nay vectors and modify them
					unwrapped_proposal.number_of_votes.aye.retain(|x| x != &who);
					unwrapped_proposal.number_of_votes.nay.try_push(who.clone()).map_err(|_| Error::<T, I>::MaxOwnersReached)?;
				} else {
					return Err(Error::<T, I>::AlreadyVotedThis.into());
				}
			} else if unwrapped_proposal.number_of_votes.nay.contains(&who) {
				if actual_vote == Vote::Aye {
					// Mutably borrow nay and aye vectors and modify them
					unwrapped_proposal.number_of_votes.nay.retain(|x| x != &who);
					unwrapped_proposal.number_of_votes.aye.try_push(who.clone()).map_err(|_| Error::<T, I>::MaxOwnersReached)?;
				} else {
					return Err(Error::<T, I>::AlreadyVotedThis.into());
				}
			} else {
				// New vote: add to the appropriate list (aye or nay)
				if actual_vote == Vote::Aye {
					unwrapped_proposal.number_of_votes.aye.try_push(who.clone()).map_err(|_| Error::<T, I>::MaxOwnersReached)?;
				} else {
					unwrapped_proposal.number_of_votes.nay.try_push(who.clone()).map_err(|_| Error::<T, I>::MaxOwnersReached)?;
				}
			}

			//Update the proposal
			CrossChainProposals::<T, I>::insert(proposal_id, unwrapped_proposal);

			//Event
			Self::deposit_event(Event::CrossChainPropoposalVoteRegistered {
				proposal_id: proposal_id,
				voter: who.clone(),
				vote: actual_vote,
			});

			//Convert accountid to accountidlookup

			
			Ok(().into())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn collection_x_transfer_initiate(origin: OriginFor<T>, proposal_id: u64) -> DispatchResultWithPostInfo{
			let who = ensure_signed(origin.clone())?;
		
			let proposal = CrossChainProposals::<T, I>::get(proposal_id);
			
			// Check if proposal exists
			ensure!(proposal.is_some(), Error::<T, I>::ProposalDoesNotExist);
			//Check if owner of the collection is the one who initiated the transfer
		
			let proposal = proposal.as_ref().unwrap(); // Borrow the proposal
				
			let owner = pallet_nfts::Pallet::<T,I>::collection_owner(proposal.collection_id.clone()).ok_or(Error::<T, I>::CollectionDoesNotExist)?;
			ensure!(owner == who.clone(), Error::<T, I>::NotCollectionOwner);

			// Check if the proposal is active or not
			let block_n: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
			if block_n < proposal.end_time {
				return Err(Error::<T, I>::ProposalStillActive.into());
			}

			// Check if the proposal passed
			let number_of_votes = proposal.number_of_votes.aye.len() + proposal.number_of_votes.nay.len();
			if (proposal.number_of_votes.aye.len() < number_of_votes / 2 || proposal.number_of_votes.aye.len() == 0 && proposal.number_of_votes.nay.len() == 0) {
				// If proposal did not pass (Less than 50% of votes are aye)
				return Err(Error::<T, I>::ProposalDidNotPass.into());
			}
			else if proposal.number_of_votes.aye.len() >= number_of_votes / 2 {
				// Transfer the collection to the destination parachain
				// (Implementation for transfer goes here)

				//Get the collection metadata
				let collection_metadata;
				if pallet_nfts::CollectionMetadataOf::<T, I>::contains_key(proposal.collection_id.clone()){
					collection_metadata = pallet_nfts::CollectionMetadataOf::<T, I>::get(proposal.collection_id.clone()).unwrap();
				}
				//Get NFT metadata
				let mut nft_metadata = Vec::new();
				
				// Iterate through all items in the collection to get item ids
				let mut items = Vec::new();

				for (item_id, _item_details) in pallet_nfts::Item::<T, I>::iter_prefix(proposal.collection_id.clone()) {
					// Process or collect the item details
					// item_id is the ItemId, and item_details contains the item's details
					//Store items in an array
					items.push(item_id);
				}

				if items.is_empty() {
					//If we have no items meanwhile we might as well just transfer the collection through regular function
					//remove proposal
					CrossChainProposals::<T, I>::remove(proposal_id);
					Self::collection_x_transfer(origin, proposal.collection_id, proposal.proposed_destination_para, T::Lookup::unlookup(proposal.proposed_collection_owner.clone()), proposal.proposed_destination_config.clone())?;
				}

				for item_id in items.clone() {
					if pallet_nfts::ItemMetadataOf::<T, I>::contains_key(proposal.collection_id.clone(), item_id) {
						if let Some(nft_owner) = pallet_nfts::Pallet::<T, I>::owner(proposal.collection_id, item_id) {
							let item_details = pallet_nfts::ItemMetadataOf::<T, I>::get(proposal.collection_id.clone(), item_id).unwrap();
							let unlookup_owner = T::Lookup::unlookup(nft_owner.clone());
							nft_metadata.push((item_id, unlookup_owner.clone(), item_details));
						}
					}
				}

				let destination = proposal.proposed_destination_para.clone();
				let recipient = proposal.proposed_collection_owner.clone();
				let unlooked_recipient = T::Lookup::unlookup(recipient.clone());
				let config = proposal.proposed_destination_config.clone();

				//Send xcm, if successful, burn the collection and NFTs
				match send_xcm::<T::XcmSender>(
					(Parent, Junction::Parachain(destination.into())).into(),
					Xcm(vec![
						UnpaidExecution { weight_limit: Unlimited, check_origin: None },
						Transact {
							origin_kind: OriginKind::SovereignAccount,
							require_weight_at_most: Weight::from_parts(1_000_000_000, 64 * 1024),
							call: <T as Config<I>>::RuntimeCall::from(pallet_nfts::Call::<
								T,
								I,
							>::create {
								admin: unlooked_recipient.clone(),
								config,
							})
							.encode()
							.into(),
						},
					]),
				) {
					Ok((_hash, _cost)) => {
						//If collection was received, remove from received collections
						if ReceivedCollections::<T, I>::contains_key(proposal.collection_id.clone()) {
							ReceivedCollections::<T, I>::remove(proposal.collection_id.clone());
						}

						//Burning the NFTs
						for item_id in items.clone() {

							//Unreserve the funds of person who created NFT
							if let Some(depositor_account) = pallet_nfts::Pallet::<T, I>::item_depositor(proposal.collection_id.clone(), item_id) {
								if let Some(deposit_amount) = pallet_nfts::Pallet::<T, I>::item_deposit(proposal.collection_id.clone(), item_id) {
									T::Currency::unreserve(&depositor_account, deposit_amount);
								} 
							}

							//Remove nft from associated account
							//Get NFT owner
							let nft_owner = pallet_nfts::Pallet::<T, I>::owner(proposal.collection_id.clone(), item_id).unwrap();
							pallet_nfts::Account::<T, I>::remove((nft_owner.clone(), proposal.collection_id.clone(), item_id));

							// Remove the NFT item from storage
							pallet_nfts::Item::<T, I>::remove(proposal.collection_id.clone(), item_id);

							//Remove nft from pending swaps
							pallet_nfts::PendingSwapOf::<T, I>::remove(proposal.collection_id.clone(), item_id);

							// Remove associated metadata
							pallet_nfts::ItemMetadataOf::<T, I>::remove(proposal.collection_id.clone(), item_id);
							
							// Remove associated attributes
							pallet_nfts::ItemAttributesApprovalsOf::<T, I>::remove(proposal.collection_id.clone(), item_id);
							
							// Clear ownership records
							pallet_nfts::ItemPriceOf::<T, I>::remove(proposal.collection_id.clone(), item_id);
							
							// Remove any pending approvals
							pallet_nfts::ItemConfigOf::<T, I>::remove(proposal.collection_id.clone(), item_id);
							
						}

						//Burning the collection
						//Retrieve the collection details
		
						// Unreserve the funds
						if let Some(col_deposit) = pallet_nfts::Pallet::<T, I>::deposit(proposal.collection_id.clone()) {
							T::Currency::unreserve(&who.clone(), col_deposit);
						}

						pallet_nfts::Collection::<T, I>::remove(proposal.collection_id.clone());

						pallet_nfts::CollectionMetadataOf::<T, I>::remove(proposal.collection_id.clone());

						pallet_nfts::CollectionConfigOf::<T, I>::remove(proposal.collection_id.clone());

						pallet_nfts::CollectionRoleOf::<T, I>::remove(proposal.collection_id.clone(), who.clone());

						pallet_nfts::CollectionAccount::<T, I>::remove(who.clone(), proposal.collection_id.clone());

						// Remove proposal from proposals to free up storage space
						CrossChainProposals::<T, I>::remove(proposal_id);

						//TODO Add shortened version of proposal to the list of completed proposals so users can track where their NFTs went


						// Emit an event.
						Self::deposit_event(Event::CollectionAndNFTsTransferred {
							origin_collection_id: proposal.collection_id.clone(),
							nft_ids: items.clone(),
							destination_para_id: proposal.proposed_destination_para.clone(),
							to_address: unlooked_recipient.clone(),
						});
					},
					Err(e) => Self::deposit_event(Event::CollectionFailedToXCM {
						e,
						collection_id: proposal.collection_id.clone(),
						owner: who.clone(),
						destination: proposal.proposed_destination_para.clone(),
					}),
				}
			}
		
			Ok(().into())
		}		

		#[pallet::call_index(3)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn nft_x_transfer(origin: OriginFor<T>, origin_collection: T::CollectionId, origin_asset: T::ItemId, destination_para: ParaId, destination_account: AccountIdLookupOf<T>, destination_collection: T::CollectionId, destination_asset: T::ItemId ) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin.clone())?;

			//See if collection exists
			let collection_exists = pallet_nfts::Collection::<T, I>::contains_key(&origin_collection);
			ensure!(collection_exists, Error::<T, I>::CollectionDoesNotExist);

			//See if item exists
			let item_exists = pallet_nfts::Item::<T, I>::contains_key(&origin_collection, &origin_asset);
			ensure!(item_exists, Error::<T, I>::NFTDoesNotExist);

			//See if user owns the item
			let owner = pallet_nfts::Pallet::<T,I>::owner(origin_collection.clone(), origin_asset.clone()).ok_or(Error::<T, I>::NFTDoesNotExist)?;
			ensure!(owner == who.clone(), Error::<T, I>::NotNFTOwner);

			let mut metadata = None;
			//Get Item data
			if pallet_nfts::ItemMetadataOf::<T, I>::contains_key(&origin_collection, &origin_asset) {
				metadata = pallet_nfts::Pallet::<T, I>::item_data(origin_collection.clone(), origin_asset.clone());
			}
			else {
				metadata = Some(BoundedVec::new());
			}

			//Convert origin to AccountIdLookup
			let w = T::Lookup::unlookup(who.clone());

			//Send the asset cross-chain

			match send_xcm::<T::XcmSender>(
				(Parent, Junction::Parachain(destination_para.into())).into(),
				Xcm(vec![
					UnpaidExecution { weight_limit: Unlimited, check_origin: None },
					Transact {
						origin_kind: OriginKind::SovereignAccount,
						require_weight_at_most: Weight::from_parts(1_000_000_000, 64 * 1024),
						call: <T as Config<I>>::XcNftCall::from(Call::<
							T,
							I,
						>::parse_nft_transfer {
							origin_collection: origin_collection.clone(),
							origin_item: origin_asset.clone(),
							collection: destination_collection.clone(),
							item: destination_asset.clone(),
							mint_to: destination_account.clone(),
							data: metadata.unwrap(),
							owner: w.clone(),
							origin_chain: parachain_info::Pallet::<T>::parachain_id(),
						})
						.encode()
						.into(),
					},
				]),
			) {
				Ok((_hash, _cost)) => {
					//If in received list, burn asset and remove from received list 
					if ReceivedAssets::<T, I>::contains_key(&(origin_collection.clone(), origin_asset.clone())) {
						//Retrieve origin chain and asset
						if let Some(received) = ReceivedAssets::<T, I>::get(&(origin_collection.clone(), origin_asset.clone())).as_ref() {
							// Use `received` as a reference
							SentAssets::<T, I>::insert(
								(origin_collection.clone(), origin_asset.clone()),
								SentStruct {
									origin_para_id: received.origin_para_id,
									origin_collection_id: received.origin_collection_id,
									origin_asset_id: received.origin_asset_id,
									destination_collection_id: destination_collection.clone(),
									destination_asset_id: destination_asset.clone(),
								},
							);
						}

						//Remove from received assets
						ReceivedAssets::<T, I>::remove(&(origin_collection.clone(), origin_asset.clone()));
						//Burn the asset
						let _ = pallet_nfts::Pallet::<T, I>::burn(origin.clone(), origin_collection.clone(), origin_asset.clone());
					}
					//Else only remove metadata, config and price
					else{
						if pallet_nfts::ItemMetadataOf::<T, I>::contains_key(&origin_collection, &origin_asset) {
							pallet_nfts::ItemMetadataOf::<T, I>::remove(&origin_collection, &origin_asset);
						}
						if pallet_nfts::ItemConfigOf::<T, I>::contains_key(&origin_collection, &origin_asset) {
							pallet_nfts::ItemConfigOf::<T, I>::remove(&origin_collection, &origin_asset);
						}
						if pallet_nfts::ItemPriceOf::<T, I>::contains_key(&origin_collection, &origin_asset) {
							pallet_nfts::ItemPriceOf::<T, I>::remove(&origin_collection, &origin_asset);
						}	

						//Create sent asset
						SentAssets::<T, I>::insert((origin_collection.clone(), origin_asset.clone()), SentStruct {
							origin_para_id:parachain_info::Pallet::<T>::parachain_id(),
							origin_collection_id: origin_collection.clone(),
							origin_asset_id: origin_asset.clone(),
							destination_collection_id: destination_collection.clone(),
							destination_asset_id: destination_asset.clone(),
						});
					}
					//Emit event
					Self::deposit_event(Event::NFTTransferred {
						origin_collection_id: origin_collection.clone(),
						origin_asset_id: origin_asset.clone(),
						destination_para_id: destination_para,
						destination_collection_id: destination_collection.clone(),
						destination_asset_id: destination_asset.clone(),
						to_address: destination_account,
					});
				},
				Err(e) => Self::deposit_event(Event::CollectionFailedToXCM {
					e,
					collection_id: origin_collection.clone(),
					owner: who.clone(),
					destination: destination_para.clone(),
				}),
			}
			Ok(().into())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn nft_x_claim(origin: OriginFor<T>, origin_collection_at_destination: T::CollectionId, origin_collection_at_origin: T::CollectionId, origin_asset_at_destination: T::ItemId, current_collection: T::CollectionId ,current_asset: T::ItemId) -> DispatchResultWithPostInfo {
			//Check if user owns the asset
			let who = ensure_signed(origin.clone())?;

			//See if collection exists
			let collection_exists = pallet_nfts::Collection::<T, I>::contains_key(&current_collection);
			ensure!(collection_exists, Error::<T, I>::CollectionDoesNotExist);

			//See if origin collection exists
			let origin_collection_exists = pallet_nfts::Collection::<T, I>::contains_key(&origin_collection_at_destination);
			ensure!(origin_collection_exists, Error::<T, I>::CollectionDoesNotExist);

			//See if origin collection at origin is the same as in received collection
			let received_collections = ReceivedCollections::<T, I>::get(&origin_collection_at_destination).unwrap();
			ensure!(received_collections.origin_collection_id == origin_collection_at_origin, Error::<T, I>::WrongOriginCollectionAtOrigin);

			//See if current asset is in received assets
			let asset_exists = ReceivedAssets::<T, I>::contains_key(&(current_collection.clone(), current_asset.clone()));
			ensure!(asset_exists, Error::<T, I>::NFTNotReceived);

			//See if item in origin collection exists
			let item_exists = pallet_nfts::Item::<T, I>::contains_key(&origin_collection_at_destination, &origin_asset_at_destination);
			ensure!(item_exists, Error::<T, I>::NFTDoesNotExist);

			//See if user owns the item
			let owner = pallet_nfts::Pallet::<T,I>::owner(origin_collection_at_destination.clone(), origin_asset_at_destination.clone()).ok_or(Error::<T, I>::NFTDoesNotExist)?;
			ensure!(owner == who.clone(), Error::<T, I>::NotNFTOwner);

			//See if user owns the current asset
			let current_owner = pallet_nfts::Pallet::<T,I>::owner(current_collection.clone(), current_asset.clone()).ok_or(Error::<T, I>::NFTDoesNotExist)?;
			ensure!(current_owner == who.clone(), Error::<T, I>::NotNFTOwner);

			//Claim the asset
			//Get the asset metadata
			let mut metadata = None;
			if pallet_nfts::ItemMetadataOf::<T, I>::contains_key(current_collection.clone(), current_asset.clone()) {
				metadata = pallet_nfts::Pallet::<T, I>::item_data(current_collection.clone(), current_asset.clone());
			}
			else{
				metadata = Some(BoundedVec::new());
			}

			//Burn the current asset
			let _ = pallet_nfts::Pallet::<T, I>::burn(origin.clone(), current_collection.clone(), current_asset.clone());

			//Add the metadata to the old asset
			if metadata.is_some() {
				let _ = pallet_nfts::Pallet::<T, I>::set_metadata(origin.clone(), origin_collection_at_destination.clone(), origin_asset_at_destination.clone(), metadata.unwrap());
			}
			//Remove asset from received
			ReceivedAssets::<T, I>::remove(&(current_collection.clone(), current_asset.clone()));

			//Emit event
			Self::deposit_event(Event::NFTClaimed {
				collection_claimed_from: current_collection.clone(),
				asset_removed: current_asset.clone(),
				collection_claimed_to: origin_collection_at_destination.clone(),
				asset_claimed: origin_asset_at_destination.clone(),
			});

			Ok(().into())
		}
	
		///Updates the collection metadata cross-chain
		#[pallet::call_index(5)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn collection_x_update(origin: OriginFor<T>, destination_collection_id: T::CollectionId, destination_para: ParaId , data: BoundedVec<u8, T::StringLimit>) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			//Convert origin to accountlookupof
			let w = T::Lookup::unlookup(who.clone());

			//Send the prompt to update 
			match send_xcm::<T::XcmSender>(
				(Parent, Junction::Parachain(destination_para.into())).into(),
				Xcm(vec![
					UnpaidExecution { weight_limit: Unlimited, check_origin: None },
					Transact {
						origin_kind: OriginKind::SovereignAccount,
						require_weight_at_most: Weight::from_parts(1_000_000_000, 64 * 1024),
						call: <T as Config<I>>::XcNftCall::from(Call::<
							T,
							I,
						>::parse_collection_metadata {
							owner: w.clone(),
							collection: destination_collection_id.clone(),
							data: data.clone(),
						})
						.encode()
						.into(),
					},
				]),
			) {
				Ok((_hash, _cost)) => {
					//Emit event about sucessful metadata send
					Self::deposit_event(Event::ColMetadataSent {
						collection_id: destination_collection_id.clone(),
						proposed_data: data.clone(),
						owner: who.clone(),
						destination: destination_para.clone(),
					});

				},
				Err(e) => Self::deposit_event(Event::ColMetadataFailedToXCM {
					e,
					collection_id: destination_collection_id.clone(),
					proposed_data: data.clone(),
					owner: who.clone(),
					destination: destination_para.clone(),
				}),
			}
			Ok(().into())
		}
	
		///Updates the item metadata cross-chain
		#[pallet::call_index(6)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn nft_x_update(origin: OriginFor<T>, destination_collection_id: T::CollectionId, destination_asset_id: T::ItemId, destination_para: ParaId , data: BoundedVec<u8, T::StringLimit>) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			//Parse the origin to accountlookupof
			let w = T::Lookup::unlookup(who.clone());

			match send_xcm::<T::XcmSender>(
				(Parent, Junction::Parachain(destination_para.into())).into(),
				Xcm(vec![
					UnpaidExecution { weight_limit: Unlimited, check_origin: None },
					Transact {
						origin_kind: OriginKind::SovereignAccount,
						require_weight_at_most: Weight::from_parts(1_000_000_000, 64 * 1024),
						call: <T as Config<I>>::XcNftCall::from(Call::<
							T,
							I,
						>::parse_nft_metadata {
							owner: w.clone(),
							collection: destination_collection_id.clone(),
							item: destination_asset_id.clone(),
							data: data.clone(),
						})
						.encode()
						.into(),
					},
				]),
			) {
				Ok((_hash, _cost)) => {
					//Emit event about sucessful metadata send
					Self::deposit_event(Event::NFTMetadataSent {
						collection_id: destination_collection_id.clone(),
						asset_id: destination_asset_id.clone(),
						proposed_data: data.clone(),
						owner: who.clone(),
						destination: destination_para.clone(),
					});

				},
				Err(e) => Self::deposit_event(Event::NFTMetadataFailedToXCM {
					e,
					collection_id: destination_collection_id.clone(),
					asset_id: destination_asset_id.clone(),
					proposed_data: data.clone(),
					owner: who.clone(),
					destination: destination_para.clone(),
				}),
			}
			Ok(().into())
		}
	
		///Burns the collection cross-chain
		#[pallet::call_index(7)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn collection_x_burn(origin: OriginFor<T>, destination_collection_id: T::CollectionId, destination_para: ParaId, witnes_data: pallet_nfts::DestroyWitness) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			//Convert origin to accountlookupof
			let w = T::Lookup::unlookup(who.clone());

			match send_xcm::<T::XcmSender>(
				(Parent, Junction::Parachain(destination_para.into())).into(),
				Xcm(vec![
					UnpaidExecution { weight_limit: Unlimited, check_origin: None },
					Transact {
						origin_kind: OriginKind::SovereignAccount,
						require_weight_at_most: Weight::from_parts(1_000_000_000, 64 * 1024),
						call: <T as Config<I>>::XcNftCall::from(Call::parse_collection_burn { owner: w.clone(), collection_to_burn: destination_collection_id.clone(), witness_data: witnes_data.clone() })
						.encode()
						.into(),
					},
				]),
			) {
				Ok((_hash, _cost)) => {
					//Emit event about sucessful metadata send
					Self::deposit_event(Event::ColBurnSent {
						collection_id: destination_collection_id.clone(),
						burn_data: witnes_data.clone(),
						owner: who.clone(),
						destination: destination_para.clone(),
					});

				},
				Err(e) => Self::deposit_event(Event::ColBurnFailedToXCM {
					e,
					collection_id: destination_collection_id.clone(),
					burn_data: witnes_data.clone(),
					owner: who.clone(),
					destination: destination_para.clone(),
				}),
			}
		
			Ok(().into())
		}

		///Burns the item cross-chain
		#[pallet::call_index(8)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn nft_x_burn (origin: OriginFor<T>, destination_collection_id: T::CollectionId, destination_asset_id: T::ItemId, destination_para: ParaId) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			//Convert origin to accountlookupof
			let w = T::Lookup::unlookup(who.clone());
			
			match send_xcm::<T::XcmSender>(
				(Parent, Junction::Parachain(destination_para.into())).into(),
				Xcm(vec![
					UnpaidExecution { weight_limit: Unlimited, check_origin: None },
					Transact {
						origin_kind: OriginKind::SovereignAccount,
						require_weight_at_most: Weight::from_parts(1_000_000_000, 64 * 1024),
						call: <T as Config<I>>::XcNftCall::from(Call::<
							T,
							I,
						>::parse_nft_burn {
							owner: w.clone(),
							collection: destination_collection_id.clone(),
							item: destination_asset_id.clone(),
						})
						.encode()
						.into(),
					},
				]),
			) {
				Ok((_hash, _cost)) => {
					//Emit event about sucessful metadata send
					Self::deposit_event(Event::NFTBurnSent {
						collection_id: destination_collection_id.clone(),
						asset_id: destination_asset_id.clone(),
						owner: who.clone(),
						destination: destination_para.clone(),
					});

				},
				Err(e) => Self::deposit_event(Event::NFTBurnFailedToXCM {
					e,
					collection_id: destination_collection_id.clone(),
					asset_id: destination_asset_id.clone(),
					owner: who.clone(),
					destination: destination_para.clone(),
				}),
			}
		
			Ok(().into())
		}

		///Change collection ownership
		#[pallet::call_index(9)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn collection_x_change_owner(origin: OriginFor<T>, destination_collection_id: T::CollectionId, destination_para: ParaId, destination_account: AccountIdLookupOf<T>) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;

			match send_xcm::<T::XcmSender>(
				(Parent, Junction::Parachain(destination_para.into())).into(),
				Xcm(vec![
					UnpaidExecution { weight_limit: Unlimited, check_origin: None },
					Transact {
						origin_kind: OriginKind::SovereignAccount,
						require_weight_at_most: Weight::from_parts(1_000_000_000, 64 * 1024),
						call: <T as Config<I>>::RuntimeCall::from(pallet_nfts::Call::<
							T,
							I,
						>::transfer_ownership {
							collection: destination_collection_id.clone(),
							new_owner: destination_account.clone(),
						})
						.encode()
						.into(),
					},
				]),
			) {
				Ok((_hash, _cost)) => {
					//Emit event about sucessful metadata send
					Self::deposit_event(Event::ColOwnershipSent {
						collection_id: destination_collection_id.clone(),
						proposed_owner: destination_account.clone(),
						destination: destination_para.clone(),
					});

				},
				Err(e) => Self::deposit_event(Event::ColOwnershipFailedToXCM {
					e,
					collection_id: destination_collection_id.clone(),
					proposed_owner: destination_account.clone(),
					destination: destination_para.clone(),
				}),
			}
		
			Ok(().into())
		}

		///Change item ownership
		#[pallet::call_index(10)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn nft_x_change_owner(origin: OriginFor<T>, destination_collection_id: T::CollectionId, destination_asset_id: T::ItemId, destination_para: ParaId, destination_account: AccountIdLookupOf<T>) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			//Change who to accountlookupof
			let w = T::Lookup::unlookup(who.clone());

			match send_xcm::<T::XcmSender>(
				(Parent, Junction::Parachain(destination_para.into())).into(),
				Xcm(vec![
					UnpaidExecution { weight_limit: Unlimited, check_origin: None },
					Transact {
						origin_kind: OriginKind::SovereignAccount,
						require_weight_at_most: Weight::from_parts(1_000_000_000, 64 * 1024),
						call: <T as Config<I>>::XcNftCall::from(Call::<
							T,
							I,
						>::parse_nft_owner {
							owner: w.clone(),
							new_owner: destination_account.clone(),
							collection: destination_collection_id.clone(),
							item: destination_asset_id.clone(),
						})
						.encode()
						.into(),
					},
				]),
			) {
				Ok((_hash, _cost)) => {
					//Emit event about sucessful metadata send
					Self::deposit_event(Event::NFTOwnershipSent {
						collection_id: destination_collection_id.clone(),
						asset_id: destination_asset_id.clone(),
						proposed_owner: destination_account.clone(),
						destination: destination_para.clone(),
					});

				},
				Err(e) => Self::deposit_event(Event::NFTOwnershipFailedToXCM {
					e,
					collection_id: destination_collection_id.clone(),
					asset_id: destination_asset_id.clone(),
					proposed_owner: destination_account.clone(),
					destination: destination_para.clone(),
				}),
			}
		
			Ok(().into())
		}

		#[pallet::call_index(11)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn parse_collection_empty(origin: OriginFor<T> ,origin_collection: T::CollectionId, receiving_account: AccountIdLookupOf<T>,  config: CollectionConfigFor<T, I>, origin_para: ParaId) -> DispatchResultWithPostInfo {
			//Create collection
			match pallet_nfts::Pallet::<T, I>::create(
				origin.clone(),
				receiving_account.clone(),
				config.clone()
			) {
				Ok(_) => {

					// Emit event
					Self::deposit_event(Event::CollectionReceived {
						origin_collection_id: origin_collection.clone(),
						received_collection_id: origin_collection.clone(),
						to_address: receiving_account,
					});
				},
				Err(e) => {
					// Deposit event indicating failure to create collection
					Self::deposit_event(Event::CollectionCreationFailed {
						owner: receiving_account.clone(),
						error: e,
					});
				}
			}

			Ok(().into())
		}


		#[pallet::call_index(12)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn parse_collection_burn(origin: OriginFor<T>, owner: AccountIdLookupOf<T>, collection_to_burn: T::CollectionId, witness_data: pallet_nfts::DestroyWitness) -> DispatchResultWithPostInfo {
			//Burn the collection

			//Receiving account to account id
			let receiving_account_id = T::Lookup::lookup(owner.clone())
			.map_err(|_| Error::<T, I>::AccountLookupFailed)?;
	
			// Convert AccountId to Signed Origin
			let signed_origin: OriginFor<T> = system::RawOrigin::Signed(receiving_account_id.clone()).into();

			match pallet_nfts::Pallet::<T, I>::destroy(signed_origin.clone(), collection_to_burn.clone(), witness_data.clone()) {
				Ok(_) => {
				},
				Err(e) => {
					// Deposit event indicating failure to burn collection
					Self::deposit_event(Event::CollectionBurnFailed {
						owner: owner.clone(),
						collection_id: collection_to_burn.clone(),
						error: e,
					});
				}
			}

			Ok(().into())
		}

		#[pallet::call_index(13)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn parse_collection_metadata(origin: OriginFor<T>, owner: AccountIdLookupOf<T> , collection: T::CollectionId, data: BoundedVec<u8, T::StringLimit>) -> DispatchResultWithPostInfo {
			
			//Receiving account to account id
			let receiving_account_id = T::Lookup::lookup(owner.clone())
			.map_err(|_| Error::<T, I>::AccountLookupFailed)?;
	
			// Convert AccountId to Signed Origin
			let signed_origin: OriginFor<T> = system::RawOrigin::Signed(receiving_account_id.clone()).into();

			//Set the collection metadata
			match pallet_nfts::Pallet::<T, I>::set_collection_metadata(signed_origin.clone(), collection.clone(), data.clone()) {
				Ok(_) => {
				},
				Err(e) => {
					// Deposit event indicating failure to set metadata
					Self::deposit_event(Event::CollectionMetadataSetFailed {
						collection_id: collection.clone(),
						owner: owner.clone(),
						error: e,
					});
				}
			}

			Ok(().into())
		}

		#[pallet::call_index(14)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn parse_collection_owner(origin: OriginFor<T>, owner: AccountIdLookupOf<T>, new_owner: AccountIdLookupOf<T>, collection: T::CollectionId) -> DispatchResultWithPostInfo {
			
			//Receiving account to account id
			let receiving_account_id = T::Lookup::lookup(owner.clone())
			.map_err(|_| Error::<T, I>::AccountLookupFailed)?;
	
			// Convert AccountId to Signed Origin
			let signed_origin: OriginFor<T> = system::RawOrigin::Signed(receiving_account_id.clone()).into();

			//Change the collection owner
			match pallet_nfts::Pallet::<T, I>::transfer_ownership(signed_origin.clone(), collection.clone(), new_owner.clone()) {
				Ok(_) => {
				},
				Err(e) => {
					// Deposit event indicating failure to transfer ownership
					Self::deposit_event(Event::CollectionOwnershipTransferFailed {
						collection_id: collection.clone(),
						owner: owner.clone(),
						error: e,
					});
				}
			}

			Ok(().into())
		}

		#[pallet::call_index(15)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn parse_nft_burn(origin: OriginFor<T>, owner: AccountIdLookupOf<T>, collection: T::CollectionId, item: T::ItemId) -> DispatchResultWithPostInfo {
			
			//Receiving account to account id
			let receiving_account_id = T::Lookup::lookup(owner.clone())
			.map_err(|_| Error::<T, I>::AccountLookupFailed)?;
	
			// Convert AccountId to Signed Origin
			let signed_origin: OriginFor<T> = system::RawOrigin::Signed(receiving_account_id.clone()).into();

			//Burn the NFT
			match pallet_nfts::Pallet::<T, I>::burn(signed_origin.clone(), collection.clone(), item.clone()) {
				Ok(_) => {
				},
				Err(e) => {
					// Deposit event indicating failure to burn NFT
					Self::deposit_event(Event::NFTBurnFailed {
						collection_id: collection.clone(),
						asset_id: item.clone(),
						owner: owner.clone(),
						error: e,
					});
				}
			}

			Ok(().into())
		}

		#[pallet::call_index(16)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn parse_nft_metadata(origin: OriginFor<T>, owner: AccountIdLookupOf<T>, collection: T::CollectionId, item: T::ItemId, data: BoundedVec<u8, T::StringLimit>) -> DispatchResultWithPostInfo {
			
			//Receiving account to account id
			let receiving_account_id = T::Lookup::lookup(owner.clone())
			.map_err(|_| Error::<T, I>::AccountLookupFailed)?;
	
			// Convert AccountId to Signed Origin
			let signed_origin: OriginFor<T> = system::RawOrigin::Signed(receiving_account_id.clone()).into();

			//Set the NFT metadata
			match pallet_nfts::Pallet::<T, I>::set_metadata(signed_origin.clone(), collection.clone(), item.clone(), data.clone()) {
				Ok(_) => {
				},
				Err(e) => {
					// Deposit event indicating failure to set metadata
					Self::deposit_event(Event::NFTMetadataSetFailed {
						collection_id: collection.clone(),
						asset_id: item.clone(),
						owner: owner.clone(),
						error: e,
					});
				}
			}

			Ok(().into())
		}

		#[pallet::call_index(17)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn parse_nft_owner(origin: OriginFor<T>, owner: AccountIdLookupOf<T>, new_owner: AccountIdLookupOf<T>, collection: T::CollectionId, item: T::ItemId) -> DispatchResultWithPostInfo {
			
			//Receiving account to account id
			let receiving_account_id = T::Lookup::lookup(owner.clone())
			.map_err(|_| Error::<T, I>::AccountLookupFailed)?;
	
			// Convert AccountId to Signed Origin
			let signed_origin: OriginFor<T> = system::RawOrigin::Signed(receiving_account_id.clone()).into();

			//Change the NFT owner
			match pallet_nfts::Pallet::<T, I>::transfer(signed_origin.clone(), collection.clone(), item.clone(), new_owner.clone()) {
				Ok(_) => {
				},
				Err(e) => {
					// Deposit event indicating failure to transfer ownership
					Self::deposit_event(Event::NFTOwnershipTransferFailed {
						collection_id: collection.clone(),
						asset_id: item.clone(),
						owner: owner.clone(),
						error: e,
					});
				}
			}

			Ok(().into())
		}
	
		#[pallet::call_index(18)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn parse_nft_transfer( origin: OriginFor<T>, owner: AccountIdLookupOf<T>, collection: T::CollectionId, item: T::ItemId, data: BoundedVec<u8, T::StringLimit>, mint_to: AccountIdLookupOf<T>, origin_collection: T::CollectionId, origin_item: T::ItemId, origin_chain: ParaId) -> DispatchResultWithPostInfo {
			
			//Check if the collection exists
			let collection_exists = pallet_nfts::Collection::<T, I>::contains_key(&collection);
			ensure!(collection_exists, Error::<T, I>::CollectionDoesNotExist);

			//Check if the item exists
			let item_exists = pallet_nfts::Item::<T, I>::contains_key(&collection, &item);
			ensure!(!item_exists, Error::<T, I>::NFTExists);

			//Check if not in receiving assets
			let received_exists = ReceivedAssets::<T, I>::contains_key(&(collection.clone(), item.clone()));
			ensure!(!received_exists, Error::<T, I>::NFTAlreadyReceived);

			//Convert accountidlookupof to accountid
			let who = T::Lookup::lookup(owner.clone())?;

			//Check if the owner owns the collection
			let col_owner = pallet_nfts::Pallet::<T, I>::collection_owner(collection);
			ensure!(col_owner.unwrap()==who.clone(), Error::<T, I>::NotCollectionOwner);

			//Receiving account to account id
			let receiving_account_id = T::Lookup::lookup(owner.clone())
			.map_err(|_| Error::<T, I>::AccountLookupFailed)?;

			// Convert AccountId to Signed Origin
			let signed_origin: OriginFor<T> = system::RawOrigin::Signed(receiving_account_id.clone()).into();

			let sent_exists = SentAssets::<T, I>::contains_key(&(collection.clone(), item.clone()));
			if sent_exists {
				//This means, that this is the NFT origin chain, we also check if ParaId matches
					// Retrieve the sent asset and store it in a variable
					let sent_asset = SentAssets::<T, I>::get(&(collection.clone(), item.clone()));

					// Ensure the sent asset exists and store a reference to it
					let sent = sent_asset.as_ref().ok_or(Error::<T, I>::NotSent)?;

					// Now you can use the reference safely
					if sent.origin_para_id == parachain_info::Pallet::<T>::parachain_id() {


					//We know we are the origin chain, we can add only metadata
					match pallet_nfts::Pallet::<T, I>::set_metadata(signed_origin.clone(), collection.clone(), item.clone(), data.clone()) {
						Ok(_) => {
						},
						Err(e) => {
							// Deposit event indicating failure to set metadata
							Self::deposit_event(Event::NFTMetadataSetFailed {
								collection_id: collection.clone(),
								asset_id: item.clone(),
								owner: owner.clone(),
								error: e,
							});
						}
					}

					//We also remove sent assets and received assets
					SentAssets::<T, I>::remove(&(origin_collection.clone(), origin_item.clone()));

					//convert mint to into accountid
					let mint_to_accid = T::Lookup::lookup(mint_to.clone())?;

					//We emit event about return to origin chain
					Self::deposit_event(Event::NFTReturnedToOrigin {
						origin_collection_id: origin_collection.clone(),
						origin_asset_id: origin_item.clone(),
						returned_from_collection_id: collection.clone(),
						returned_from_asset_id: item.clone(),
						to_address: mint_to_accid.clone(),
					});

					return Ok(().into())
				}
				else {
					//We know, that we return to chain that sent this NFT once already
					//We proceed as normal, with the only difference being, that we remove the item from sent assets
					SentAssets::<T, I>::remove(&(collection.clone(), item.clone()));
				}
			}

			//Mint the NFT
			match pallet_nfts::Pallet::<T, I>::mint(signed_origin.clone(), collection.clone(), item.clone(), mint_to.clone(), None) {
				Ok(_) => {
				},
				Err(e) => {
					// Deposit event indicating failure to mint NFT
					Self::deposit_event(Event::NFTMintFailed {
						collection_id: collection.clone(),
						asset_id: item.clone(),
						owner: owner.clone(),
						error: e,
					});
				}
			}

			if !data.is_empty(){
				match pallet_nfts::Pallet::<T, I>::set_metadata(signed_origin.clone(), collection.clone(), item.clone(), data.clone()) {
					Ok(_) => {
					},
					Err(e) => {
						// Deposit event indicating failure to set metadata
						Self::deposit_event(Event::NFTMetadataSetFailed {
							collection_id: collection.clone(),
							asset_id: item.clone(),
							owner: owner.clone(),
							error: e,
						});
					}
				}
		}

			//Check if the NFT was minted if storage contains the item
			let item_exists = pallet_nfts::Item::<T, I>::contains_key(&collection, &item);
			ensure!(item_exists, Error::<T, I>::NFTDoesNotExist);

			//Add the item to the received item storage
			ReceivedAssets::<T, I>::insert((collection.clone(), item.clone()), ReceivedStruct {
				origin_para_id: origin_chain.clone(),
				origin_collection_id: origin_collection.clone(),
				origin_asset_id: origin_item.clone(),
				received_collection_id: collection.clone(),
				received_asset_id: item.clone(),
			});
			
			//Emit event
			Self::deposit_event(Event::NFTReceived {
				origin_collection_id: origin_collection.clone(),
				origin_asset_id: origin_item.clone(),
				received_collection_id: collection.clone(),
				received_asset_id: item.clone(),
				to_address: mint_to.clone(),
			});


			Ok(().into())
			
		}

		#[pallet::call_index(19)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn parse_collection_same_owner(origin: OriginFor<T>, owner: AccountIdLookupOf<T>, config: CollectionConfigFor<T, I>, collection_metadata: BoundedVec<u8, T::StringLimit> ,nfts: Vec<(T::ItemId, BoundedVec<u8, T::StringLimit>)>, origin_para: ParaId, origin_collection_id: T::CollectionId) -> DispatchResultWithPostInfo {

			//Convert accountidlookupof to accountid
			let who = T::Lookup::lookup(owner.clone())?;

			// Convert AccountId to Signed Origin
			let signed_origin: OriginFor<T> = system::RawOrigin::Signed(who.clone()).into();

			//Get next collection id
			let mut next_collection_id = pallet_nfts::NextCollectionId::<T, I>::get()
			.or(T::CollectionId::initial_value()).unwrap();

			let collection = pallet_nfts::Pallet::<T, I>::create(
				signed_origin.clone(),
				owner.clone(),
				config.clone()
			)?;		

			let mut user_collection = next_collection_id.clone();
			//just to be sure, check if user is collection owner
			let col_owner = pallet_nfts::Pallet::<T, I>::collection_owner(next_collection_id.clone());
			if col_owner.unwrap()!=who.clone(){
				//Get current next_collection_id
				let current_next_collection_id = pallet_nfts::NextCollectionId::<T, I>::get().ok_or(Error::<T, I>::NoNextCollectionId)?;
				//Go from next_collection_id to current_next_collection_id and try to find the collection that is owned by the user
				while next_collection_id!=current_next_collection_id{
					let col_owner = pallet_nfts::Pallet::<T, I>::collection_owner(next_collection_id);
					if col_owner.unwrap()==who.clone(){
						//We have users collection
						user_collection = next_collection_id.clone();
						break;
					}
					next_collection_id = next_collection_id.increment().ok_or(Error::<T, I>::NoNextCollectionId)?;
				}
			}

			//Set the collection metadata if present
			if !collection_metadata.is_empty(){
				match pallet_nfts::Pallet::<T, I>::set_collection_metadata(signed_origin.clone(), user_collection.clone(), collection_metadata.clone()) {
					Ok(_) => {
					},
					Err(e) => {
						// Deposit event indicating failure to set metadata
						Self::deposit_event(Event::CollectionMetadataSetFailed {
							collection_id: user_collection.clone(),
							owner: owner.clone(),
							error: e,
						});
					}
				}
			}
			//Mint the NFTs
			//Iterate through vector of nfts
			for nft in nfts.clone() {
				let item = nft.0;
				let data = nft.1;

				//Mint the NFT
				match pallet_nfts::Pallet::<T, I>::mint(signed_origin.clone(), user_collection.clone(), item.clone(), owner.clone(), None) {
					Ok(_) => {
					},
					Err(e) => {
						// Deposit event indicating failure to mint NFT
						Self::deposit_event(Event::NFTMintFailed {
							collection_id: user_collection.clone(),
							asset_id: item.clone(),
							owner: owner.clone(),
							error: e,
						});
					}
				}
				//If empty metadata, skip
				if data.is_empty(){

				}else{
					match pallet_nfts::Pallet::<T, I>::set_metadata(signed_origin.clone(), user_collection.clone(), item.clone(), data.clone()) {
						Ok(_) => {
						},
						Err(e) => {
							// Deposit event indicating failure to set metadata
							Self::deposit_event(Event::NFTMetadataSetFailed {
								collection_id: user_collection.clone(),
								asset_id: item.clone(),
								owner: owner.clone(),
								error: e,
							});
						}
					}
				}

				//Check if the NFT was minted if storage contains the item
				let item_exists = pallet_nfts::Item::<T, I>::contains_key(&user_collection, &item);
				ensure!(item_exists, Error::<T, I>::NFTDoesNotExist);
				
			}

			//Add collection to received collections
			ReceivedCollections::<T, I>::insert(user_collection.clone(), ReceivedCols {
				origin_para_id: origin_para.clone(),
				origin_collection_id: origin_collection_id.clone(),
				received_collection_id: user_collection.clone(),
			});
			
			//If all went up to this point, emit event about successful cross-chain operation
			Self::deposit_event(Event::CollectionWithNftsReceived {
				collection_id: user_collection.clone(),
				items: nfts.clone(),
			});


			Ok(().into())
		}
	
		#[pallet::call_index(20)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn parse_collection_multiple_owners( origin: OriginFor<T>, owner: AccountIdLookupOf<T>, config: CollectionConfigFor<T, I>, collection_metadata: BoundedVec<u8, T::StringLimit> ,nfts: Vec<(T::ItemId, AccountIdLookupOf<T> ,BoundedVec<u8, T::StringLimit>)>, origin_para: ParaId, origin_collection_id: T::CollectionId) -> DispatchResultWithPostInfo {
			Ok(().into())

		}
	}
}

