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
//! - Transfering non-fungible assets cross-chain: **nftXtransfer**
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

	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::{CheckedAdd, One};
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*, DefaultNoBound, sp_runtime::traits::Hash, 
		traits::{Currency, LockableCurrency, ReservableCurrency}};
	use frame_support::traits::PalletInfo;
	use codec::Encode;
	use cumulus_primitives_core::{ParaId};
	use scale_info::prelude::vec;
	use sp_std::prelude::*;
	use xcm::latest::prelude::*;
	use pallet_nfts::{AttributeNamespace, Call as NftsCall, ItemDetails};
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
	pub trait Config<I: 'static = ()>: frame_system::Config + pallet_nfts::Config<I> {
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

		/// Max amount of aye and nay
		type MaxVotes: Get<u32>;
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

	#[pallet::storage]
	#[pallet::getter(fn sent_assets)]
	pub type SentAssets<T: Config<I>, I: 'static = ()> = StorageMap<
    _,
    Blake2_128Concat,
    (T::CollectionId, T::ItemId),  // Tuple (origin_collection_id, origin_asset_id, pallet_type)
    SentStruct<T, I>,       // Sent assets structure
	>;

	#[pallet::storage]
	#[pallet::getter(fn received_assets)]
	pub type ReceivedAssets<T: Config<I>, I: 'static = ()> = StorageMap<
    _,
    Blake2_128Concat,
    (T::CollectionId, T::ItemId),  // Tuple (received_collection_id, received_asset_id, pallet_type)
    ReceivedStruct<T, I>,   // Received assets structure
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

		/// Event emitted when cross-chain transfer fails
		CollectionTransferProposalCreated {
			proposal_id: u64,
			collection_id: T::CollectionId,
			owner: T::AccountId,
			destination: ParaId,
		},

		CollectionAndNFTsTransferred {
			origin_collection_id: T::CollectionId,
			nft_ids: Vec<T::ItemId>,
			destination_para_id: ParachainID,
			to_address: AccountIdLookupOf<T>,
		},

		CrossChainPropoposalVoteRegistered{
			proposal_id: u64,
			voter: T::AccountId,
			vote: Vote,
		},

		TestEvent{
			proposal: T::CollectionId,
			collection: T::CollectionId,
		}
	}

	/// <https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html#event-and-error>
	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Error names should be descriptive.
		CollectionDoesNotExist,
		ProposalAlreadyExists,
		NotCollectionOwner,
		ProposalExpired,
		AlreadyVotedThis,
		MaxOwnersReached,
		NotNFTOwner,
	}

	//#[pallet::hooks]
	//impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	/// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	/// These functions materialize as "extrinsics", which are often compared to transactions.
	/// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	/// <https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html#dispatchables>
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
						//Burning the collection
						pallet_nfts::Collection::<T, I>::remove(&origin_collection);

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
								owner: who.clone(),
								destination: destination_para,							
							});
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
						//Burning the NFTs
						for item_id in items.clone() {
							pallet_nfts::Pallet::<T, I>::burn(origin.clone(), origin_collection.clone(), item_id);
						}
						//Burning the collection
						pallet_nfts::Collection::<T, I>::remove(&origin_collection);

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

		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::call_index(1)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn collectionXtransferVote(origin: OriginFor<T>, proposal_id: u64, actual_vote: Vote) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			let proposal = CrossChainProposals::<T, I>::get(proposal_id);
			ensure!(proposal.is_some(), Error::<T, I>::ProposalAlreadyExists); // Check before unwrapping
		
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
				if unwrapped_proposal.number_of_votes.aye.len() < number_of_votes / 2 {
					CrossChainProposals::<T, I>::remove(proposal_id);
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
			
			Ok(().into())
		}

	}
}
