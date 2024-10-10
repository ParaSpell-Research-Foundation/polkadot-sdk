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
	use sp_runtime::traits::{CheckedAdd, One};
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*, sp_runtime::traits::Hash, traits::{Currency, LockableCurrency, ReservableCurrency}, DefaultNoBound};
	use frame_support::traits::PalletInfo;
	use codec::Encode;
	use cumulus_primitives_core::{ParaId};
	use scale_info::prelude::vec;
	use sp_std::prelude::*;
	use xcm::latest::prelude::*;
	use pallet_nfts::{AttributeNamespace, Call as NftsCall, ItemDetails, ItemMetadata, CollectionDetails, DestroyWitness};
	use core::marker::PhantomData;
	
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
			owner: T::AccountId,
			destination: ParaId,
		},
	
		/// Event emitted when collection metadata is transferred cross-chain
		ColMetadataSent{
			collection_id: T::CollectionId,
			owner: T::AccountId,
			destination: ParaId,
		},

		/// Event emitted when cross-chain NFT metadata transfer fails
		NFTMetadataFailedToXCM {
			e: SendError,
			collection_id: T::CollectionId,
			asset_id: T::ItemId,
			owner: T::AccountId,
			destination: ParaId,
		},
			
		/// Event emitted when NFT metadata is transferred cross-chain
		NFTMetadataSent{
			collection_id: T::CollectionId,
			asset_id: T::ItemId,
			owner: T::AccountId,
			destination: ParaId,
		},

		/// Event emitted when cross-chain collection burn call transfer fails
		ColBurnFailedToXCM {
			e: SendError,
			collection_id: T::CollectionId,
			owner: T::AccountId,
			destination: ParaId,
		},
					
		/// Event emitted when collection burn call is transferred cross-chain
		ColBurnSent{
			collection_id: T::CollectionId,
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
	}

	/// <https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html#event-and-error>
	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Error names should be descriptive.
		CollectionDoesNotExist,
		NFTDoesNotExist,
		ProposalAlreadyExists,
		NotCollectionOwner,
		ProposalExpired,
		ProposalStillActive,
		ProposalDoesNotExist,
		ProposalDidNotPass,
		AlreadyVotedThis,
		MaxOwnersReached,
		NotNFTOwner,
		NFTNotReceived,
	}

	//#[pallet::hooks]
	//impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}


	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {

		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn collectionXtransfer(origin: OriginFor<T>, origin_collection: T::CollectionId, destination_para: ParaId, destination_account: AccountIdLookupOf<T>, config: CollectionConfigFor<T, I> ) -> DispatchResultWithPostInfo {
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
							call: <T as Config<I>>::RuntimeCall::from(pallet_nfts::Call::<
								T,
								I,
							>::create {
								admin: destination_account.clone(),
								config,
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
				let mut collection_metadata;
				if pallet_nfts::CollectionMetadataOf::<T, I>::contains_key(&origin_collection){
					collection_metadata = pallet_nfts::CollectionMetadataOf::<T, I>::get(&origin_collection).unwrap();
				}

				//Get NFT configs
				let mut nft_metadata = Vec::new();
				for item_id in items.clone() {
					if pallet_nfts::ItemMetadataOf::<T, I>::contains_key(&origin_collection, item_id) {
						let item_details = pallet_nfts::ItemMetadataOf::<T, I>::get(&origin_collection, item_id).unwrap();
						nft_metadata.push(item_details);
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
							call: <T as Config<I>>::RuntimeCall::from(pallet_nfts::Call::<
								T,
								I,
							>::create {
								admin: destination_account.clone(),
								config,
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
							pallet_nfts::Pallet::<T, I>::burn(origin.clone(), origin_collection.clone(), item_id);
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
		pub fn collectionXtransferVote(origin: OriginFor<T>, proposal_id: u64, actual_vote: Vote) -> DispatchResultWithPostInfo {
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
		pub fn collectionXtransferInitiate(origin: OriginFor<T>, proposal_id: u64) -> DispatchResultWithPostInfo{
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
				let mut collection_metadata;
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
					Self::collectionXtransfer(origin, proposal.collection_id, proposal.proposed_destination_para, T::Lookup::unlookup(proposal.proposed_collection_owner.clone()), proposal.proposed_destination_config.clone())?;
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
		pub fn nftXtransfer(origin: OriginFor<T>, origin_collection: T::CollectionId, origin_asset: T::ItemId, destination_para: ParaId, destination_account: AccountIdLookupOf<T>, destination_collection: T::CollectionId, destination_asset: T::ItemId ) -> DispatchResultWithPostInfo {
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
				metadata = pallet_nfts::ItemMetadataOf::<T, I>::get(&origin_collection, &origin_asset);
			}


			//Send the asset cross-chain

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
						>::mint {
							collection: destination_collection.clone(),
							item: destination_asset.clone(),
							mint_to: destination_account.clone(),
							witness_data: None,
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
						pallet_nfts::Pallet::<T, I>::burn(origin.clone(), origin_collection.clone(), origin_asset.clone());
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
		pub fn nftXclaim(origin: OriginFor<T>, origin_collection: T::CollectionId, origin_asset: T::ItemId, current_collection: T::CollectionId ,current_asset: T::ItemId) -> DispatchResultWithPostInfo {
			//Check if user owns the asset
			let who = ensure_signed(origin.clone())?;

			//See if collection exists
			let collection_exists = pallet_nfts::Collection::<T, I>::contains_key(&current_collection);
			ensure!(collection_exists, Error::<T, I>::CollectionDoesNotExist);

			//See if origin collection exists
			let received_collection = ReceivedCollections::<T, I>::get(origin_collection.clone()).unwrap().received_collection_id;
			let origin_collection_exists = pallet_nfts::Collection::<T, I>::contains_key(&origin_collection);
			ensure!(origin_collection_exists, Error::<T, I>::CollectionDoesNotExist);

			//See if current asset is in received assets
			let asset_exists = ReceivedAssets::<T, I>::contains_key(&(current_collection.clone(), current_asset.clone()));
			ensure!(asset_exists, Error::<T, I>::NFTNotReceived);

			//See if item in origin collection exists
			let item_exists = pallet_nfts::Item::<T, I>::contains_key(&received_collection, &origin_asset);
			ensure!(item_exists, Error::<T, I>::NFTDoesNotExist);

			//See if user owns the item
			let owner = pallet_nfts::Pallet::<T,I>::owner(received_collection.clone(), origin_asset.clone()).ok_or(Error::<T, I>::NFTDoesNotExist)?;
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

			//Burn the current asset
			pallet_nfts::Pallet::<T, I>::burn(origin.clone(), current_collection.clone(), current_asset.clone());

			//Add the metadata to the old asset
			pallet_nfts::Pallet::<T, I>::set_metadata(origin.clone(), received_collection.clone(), origin_asset.clone(), metadata.unwrap());

			//Remove asset from received
			ReceivedAssets::<T, I>::remove(&(current_collection.clone(), current_asset.clone()));

			//Emit event
			Self::deposit_event(Event::NFTClaimed {
				collection_claimed_from: current_collection.clone(),
				asset_removed: current_asset.clone(),
				collection_claimed_to: received_collection.clone(),
				asset_claimed: origin_asset.clone(),
			});

			Ok(().into())
		}
	
		///Updates the collection metadata cross-chain
		#[pallet::call_index(5)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn collectionXupdate(origin: OriginFor<T>, destination_collection_id: T::CollectionId, destination_para: ParaId , data: BoundedVec<u8, T::StringLimit>) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			//Send the prompt to update 
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
						>::set_collection_metadata {
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
						owner: who.clone(),
						destination: destination_para.clone(),
					});

				},
				Err(e) => Self::deposit_event(Event::ColMetadataFailedToXCM {
					e,
					collection_id: destination_collection_id.clone(),
					owner: who.clone(),
					destination: destination_para.clone(),
				}),
			}
			Ok(().into())
		}
	
		///Updates the item metadata cross-chain
		#[pallet::call_index(6)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn nftXupdate(origin: OriginFor<T>, destination_collection_id: T::CollectionId, destination_asset_id: T::ItemId, destination_para: ParaId , data: BoundedVec<u8, T::StringLimit>) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

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
						>::set_metadata {
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
						owner: who.clone(),
						destination: destination_para.clone(),
					});

				},
				Err(e) => Self::deposit_event(Event::NFTMetadataFailedToXCM {
					e,
					collection_id: destination_collection_id.clone(),
					asset_id: destination_asset_id.clone(),
					owner: who.clone(),
					destination: destination_para.clone(),
				}),
			}
			Ok(().into())
		}
	
		///Burns the collection cross-chain
		#[pallet::call_index(7)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn collectionXburn(origin: OriginFor<T>, destination_collection_id: T::CollectionId, destination_para: ParaId, witnes_data: pallet_nfts::DestroyWitness) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

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
						>::destroy {
							collection: destination_collection_id.clone(),
							witness: witnes_data.clone(),
						})
						.encode()
						.into(),
					},
				]),
			) {
				Ok((_hash, _cost)) => {
					//Emit event about sucessful metadata send
					Self::deposit_event(Event::ColBurnSent {
						collection_id: destination_collection_id.clone(),
						owner: who.clone(),
						destination: destination_para.clone(),
					});

				},
				Err(e) => Self::deposit_event(Event::ColBurnFailedToXCM {
					e,
					collection_id: destination_collection_id.clone(),
					owner: who.clone(),
					destination: destination_para.clone(),
				}),
			}
		
			Ok(().into())
		}

		///Burns the item cross-chain
		#[pallet::call_index(8)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn nftXburn (origin: OriginFor<T>, destination_collection_id: T::CollectionId, destination_asset_id: T::ItemId, destination_para: ParaId) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			
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
						>::burn {
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
	}
}
