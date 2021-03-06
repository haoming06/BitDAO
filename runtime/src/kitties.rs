use frame_support::{
	decl_module, decl_storage, decl_event, decl_error, ensure, StorageValue, StorageMap,print,
	Parameter, traits::{Randomness, Currency, ExistenceRequirement}
};
// use rstd::convert::{Into, TryFrom, TryInto};
use sp_runtime::{traits::{SimpleArithmetic, Bounded, Member}, DispatchError,	transaction_validity::{
	TransactionLongevity, TransactionPriority, TransactionValidity, UnknownTransaction,
	ValidTransaction,
	},DispatchResult as dispatch_result};
use codec::{Encode, EncodeLike, Decode, Output, Input};
use sp_io::hashing::blake2_128;
use system::{ensure_signed, ensure_root,ensure_none};
use sp_std::result;
use crate::linked_item::{LinkedList, LinkedItem};
use system::{offchain::SubmitUnsignedTransaction};
use sp_std::{
	convert::{Into, TryInto},
	prelude::*,
	result::Result,
	vec::Vec,
};
#[cfg(feature = "std")]
use serde::{Serialize, Deserialize};
use sp_runtime::traits::{Hash, BlakeTwo256};
use sp_runtime::RandomNumberGenerator;
pub trait Trait: system::Trait+timestamp::Trait+sudo::Trait{
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	type KittyIndex: Parameter + Member + SimpleArithmetic + Bounded + Default + Copy+Into<u32>;
	type Currency: Currency<Self::AccountId>;
	type Randomness: Randomness<Self::Hash>;
	type Call: From<Call<Self>>;
	type SubmitTransaction: SubmitUnsignedTransaction<Self, <Self as Trait>::Call>;
}
pub type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;
type WeaponryIndex = u32;
pub struct Kitty(pub [u8; 16]);

#[derive(Encode,Decode,Default,Clone,PartialEq)]
pub struct WildAnimal {
	pub hp:u32, 
	pub ce:u32,
}

#[derive(Encode,Decode,PartialEq,Eq,Clone,Debug)]
pub enum BattleType{
	WILD, 
	KITTY,
}

#[derive(Encode,Decode,Default,Clone,PartialEq)]
pub struct KittyAttr<BlockNumber> {
	pub hp:u32, 
	pub exp:u32,
	pub ce:u32,
	pub battle_begin: Option<BlockNumber>,
	pub battle_end: Option<BlockNumber>,
	pub battle_type: Option<BattleType>,
}
#[derive(Encode,Decode,Clone,PartialEq,Eq,Copy,Debug)]
#[cfg_attr(feature = "std", derive( Serialize, Deserialize))]
pub enum WeaponryKind{
	HELMET,//头盔
	ARMOR, //铠甲
	WEAPON, //武器
	SHOES,//鞋子
}
//疯狂面具,吸血面具,刃甲,强袭装甲,羊刀,圣剑 精灵皮靴,飞鞋
#[derive(Encode,Decode,Clone,PartialEq,Debug)]
#[cfg_attr(feature = "std", derive( Serialize, Deserialize))]
pub struct Weaponry<BalanceOf>{
	pub name:Vec<u8>,
	pub kind:WeaponryKind,
	pub ce:u32,
	pub price:BalanceOf,
}

impl Encode for Kitty {
	fn encode_to<T: Output>(&self, output: &mut T) {
		output.push(&self.0);
	}
}

impl EncodeLike for Kitty {}

impl Decode for Kitty {
	fn decode<I: Input>(input: &mut I) -> Result<Self, codec::Error> {
		Ok(Kitty(Decode::decode(input)?))
	}
}

type KittyLinkedItem<T> = LinkedItem<<T as Trait>::KittyIndex>;
type OwnedKittiesList<T> = LinkedList<OwnedKitties<T>, <T as system::Trait>::AccountId, <T as Trait>::KittyIndex>;
// type WeaponryLinkedItem = LinkedItem<WeaponryIndex>;
// type WeaponrysList<T> = LinkedList<WeaponryIndexList,<T as system::Trait>::AccountId, WeaponryIndex>;

decl_storage! {
	trait Store for Module<T: Trait> as Kitties {
		/// Stores all the kitties, key is the kitty id / index
		pub Kitties get(fn kitties): map T::KittyIndex => Option<Kitty>;

		pub WildAnimalsCount get(fn wild_animals_count): u32;
		
		pub WildAnimals get(fn wild_animals): map u32 => Option<WildAnimal>;
		/// Stores the total number of kitties. i.e. the next kitty index
		pub KittiesCount get(fn kitties_count): T::KittyIndex;

		pub OwnedKitties get(fn owned_kitties): map (T::AccountId, Option<T::KittyIndex>) => Option<KittyLinkedItem<T>>;

		/// Get kitty owner
		pub KittyOwners get(fn kitty_owner): map T::KittyIndex => Option<T::AccountId>;
		/// Get kitty price. None means not for sale.
		pub KittyPrices get(fn kitty_price): map T::KittyIndex => Option<BalanceOf<T>>;

		pub KittyAttrs get(fn kitty_attrs): map  T::KittyIndex => KittyAttr<T::BlockNumber>;

		//系统武器商店
		pub Weaponrys get(fn weaponrys) config() :map WeaponryIndex => Option<Weaponry<BalanceOf<T>>>;

		//武器数量
		pub WeaponrysCount get(fn weaponrys_count) config() : WeaponryIndex;

		//每猫至多装配四个武器
		pub KittyWeaponrys get(fn kitty_weaponrys):map  (T::KittyIndex,WeaponryKind) =>Option<WeaponryIndex>;
	}
}

decl_event!(
	pub enum Event<T> where
		<T as system::Trait>::AccountId,
		<T as Trait>::KittyIndex,
		Balance = BalanceOf<T>,
	{
		/// A kitty is created. (owner, kitty_id)
		Created(AccountId, KittyIndex),
		/// A kitty is transferred. (from, to, kitty_id)
		Transferred(AccountId, AccountId, KittyIndex),
		/// A kitty is available for sale. (owner, kitty_id, price)
		Ask(AccountId, KittyIndex, Option<Balance>),
		/// A kitty is sold. (from, to, kitty_id, price)
		Sold(AccountId, AccountId, KittyIndex, Balance),
		/// A weaponry is created(owner,weaponry_index)
		AddWeaponry(AccountId,WeaponryIndex),
		/// Weaponry bought for kitty .(owner,kitty_id,weaponry_index,price)
		BuyWeaponry(AccountId, KittyIndex, WeaponryIndex, Balance),
		///full health .(owner,kitty_id,price)
		FullHealth(AccountId,KittyIndex,Balance),
		/// Battled (owner,kitty_1,kitty_2,win_kitty_id)
		Battled(AccountId,KittyIndex,KittyIndex,KittyIndex),
		///offchain worker lottery (kitty_id)
		Lottery(KittyIndex),
	}
);

decl_error! {
	pub enum Error for Module<T: Trait> {
		RequiresOwner,
		InvalidKittyId,
		KittyNotForSale,
		PriceTooLow,
		KittiesCountOverflow,
		RequiresDifferentParents,
		//TODO 添加错误信息
		IndexCountOverflow,
		KittyIsBattling,
		RequiresDifferentOwner,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		/// Create a new kitty
		pub fn create(origin) {
			let sender = ensure_signed(origin)?;
			let kitty_id = Self::next_kitty_id()?;

			// Generate a random 128bit value
			let dna = Self::random_value(&sender);

			// Create and store kitty
			let kitty = Kitty(dna);
			Self::insert_kitty(&sender, kitty_id, kitty);

			Self::deposit_event(RawEvent::Created(sender, kitty_id));
		}

		/// Create a new wild animal
		pub fn create_wild_animal(origin, health_point:u32, combat_effectiveness:u32) {
			ensure_root(origin)?;
			let wild_animal_id = Self::wild_animals_count();
			WildAnimalsCount::put(wild_animal_id.checked_add(1).ok_or("overflow")?);
			let wild_animal = WildAnimal{
				hp: health_point,
				ce: combat_effectiveness,
			};
			WildAnimals::insert(wild_animal_id, wild_animal);
		}

		/// Breed kitties
		pub fn breed(origin, kitty_id_1: T::KittyIndex, kitty_id_2: T::KittyIndex) {
			let sender = ensure_signed(origin)?;

			let new_kitty_id = Self::do_breed(&sender, kitty_id_1, kitty_id_2)?;

			Self::deposit_event(RawEvent::Created(sender, new_kitty_id));
		}

		/// Transfer a kitty to new owner
 		pub fn transfer(origin, to: T::AccountId, kitty_id: T::KittyIndex) {
 			let sender = ensure_signed(origin)?;

  			ensure!(<OwnedKitties<T>>::exists((&sender, Some(kitty_id))), Error::<T>::RequiresOwner);

			Self::do_transfer(&sender, &to, kitty_id);

			Self::deposit_event(RawEvent::Transferred(sender, to, kitty_id));
		}

		/// Set a price for a kitty for sale
		/// None to delist the kitty
		pub fn ask(origin, kitty_id: T::KittyIndex, price: Option<BalanceOf<T>>) {
			let sender = ensure_signed(origin)?;

			ensure!(<OwnedKitties<T>>::exists((&sender, Some(kitty_id))), Error::<T>::RequiresOwner);

			if let Some(ref price) = price {
				<KittyPrices<T>>::insert(kitty_id, price);
			} else {
				<KittyPrices<T>>::remove(kitty_id);
			}

			Self::deposit_event(RawEvent::Ask(sender, kitty_id, price));
		}

		pub fn buy(origin, kitty_id: T::KittyIndex, price: BalanceOf<T>) {
			let sender = ensure_signed(origin)?;

			let owner = Self::kitty_owner(kitty_id);
			ensure!(owner.is_some(), Error::<T>::InvalidKittyId);
			let owner = owner.unwrap();

			let kitty_price = Self::kitty_price(kitty_id);
			ensure!(kitty_price.is_some(), Error::<T>::KittyNotForSale);

			let kitty_price = kitty_price.unwrap();
			ensure!(price >= kitty_price, Error::<T>::PriceTooLow);

			T::Currency::transfer(&sender, &owner, kitty_price, ExistenceRequirement::KeepAlive)?;

			<KittyPrices<T>>::remove(kitty_id);

			Self::do_transfer(&owner, &sender, kitty_id);

			Self::deposit_event(RawEvent::Sold(owner, sender, kitty_id, kitty_price));
		}
		pub fn battle(origin,kitty_id:T::KittyIndex,target_id:T::KittyIndex){
			let sender = ensure_signed(origin)?;
			let curr_block = <system::Module<T>>::block_number();
			ensure!(<OwnedKitties<T>>::exists((&sender, Some(kitty_id))), Error::<T>::RequiresOwner);
			ensure!(Self::check_battling(kitty_id),Error::<T>::KittyIsBattling);
			ensure!(Self::kitty_owner(target_id).map(|owner| owner != sender).unwrap_or(true), Error::<T>::RequiresDifferentOwner);
			ensure!(Self::check_battling(target_id),Error::<T>::KittyIsBattling);
			let mut kitty_attr_1 = Self::kitty_attrs(kitty_id);
			let mut kitty_attr_2 = Self::kitty_attrs(target_id);
			let kitty_1_ce = Self::get_kitty_ce(&kitty_id);
			let kitty_2_ce = Self::get_kitty_ce(&target_id);
			if kitty_attr_1.hp/kitty_2_ce>=kitty_attr_2.hp/kitty_1_ce{
				kitty_attr_1.exp = kitty_attr_1.exp+kitty_attr_2.hp*kitty_2_ce;
				kitty_attr_1.hp = kitty_attr_1.hp-kitty_attr_2.hp*kitty_2_ce/10;
				kitty_attr_2.hp = 0;
				kitty_attr_2.exp = kitty_attr_2.exp.checked_sub(kitty_attr_2.exp/10).ok_or("Overflow.")?;
			}else{
				kitty_attr_2.exp = kitty_attr_2.exp+kitty_attr_1.hp*kitty_1_ce;
				kitty_attr_2.hp = kitty_attr_2.hp-kitty_attr_1.hp*kitty_1_ce/10;
				kitty_attr_1.hp = 0;
				kitty_attr_1.exp = kitty_attr_1.exp.checked_sub(kitty_attr_1.exp/10).ok_or("Overflow.")?;
			}
			kitty_attr_1.battle_begin = Some(curr_block);
			kitty_attr_1.battle_end = Some(curr_block+4.into());
			kitty_attr_2.battle_begin = Some(curr_block);
			kitty_attr_2.battle_end = Some(curr_block+4.into());
			if kitty_attr_1.hp == 0{
				Self::deposit_event(RawEvent::Battled(sender,kitty_id,target_id,target_id))
			}else{
				Self::deposit_event(RawEvent::Battled(sender,kitty_id,target_id,kitty_id));
			}
			// match kitty_attr_1.hp{
			// 	0=>Self::deposit_event(RawEvent::Battled(sender,kitty_id,target_id,target_id)),
			// 	_=>Self::deposit_event(RawEvent::Battled(sender,kitty_id,target_id,kitty_id))
			// }
			<KittyAttrs<T>>::insert(kitty_id,kitty_attr_1);
			<KittyAttrs<T>>::insert(target_id,kitty_attr_2);
		}


		pub fn battle_wild(origin,kitty_id:T::KittyIndex,wild_animal_id:u32){
			let sender = ensure_signed(origin)?;
			ensure!(<OwnedKitties<T>>::exists((&sender, Some(kitty_id))), Error::<T>::RequiresOwner);
			let ce = Self::get_kitty_ce(&kitty_id);
			let mut kitty = Self::kitty_attrs(kitty_id);
			let wild_animal = Self::wild_animals(wild_animal_id);
			let wild_animal = wild_animal.unwrap();

			let kitty_score = kitty.hp / wild_animal.ce;
			let wild_animal_score = wild_animal.hp / ce;

			if kitty_score > wild_animal_score {
				// success
				kitty.exp = kitty.exp + wild_animal.hp * wild_animal.ce;
			}

			//除了一个系数
			kitty.hp = kitty.hp - wild_animal.hp * wild_animal.ce / 10;
			<KittyAttrs<T>>::insert(kitty_id, kitty);
		}

		pub fn full_health(origin,kitty_id:T::KittyIndex){
			let sender = ensure_signed(origin)?;
			ensure!(<OwnedKitties<T>>::exists((&sender, Some(kitty_id))), Error::<T>::RequiresOwner);
			let mut kitty = Self::kitty_attrs(kitty_id);
			if kitty.hp < 100 {
				//加血
				kitty.hp = 100;
			}
			let root_key = <sudo::Module<T>>::key();
			T::Currency::transfer(&sender,&root_key, 10.into(), ExistenceRequirement::KeepAlive)?;
			<KittyAttrs<T>>::insert(kitty_id, kitty);
			Self::deposit_event(RawEvent::FullHealth(sender,kitty_id,10.into()))
		}
		//商店上新装备,需要root权限
		pub fn add_weaponry(origin,kind:WeaponryKind,name:Vec<u8>,combat_effectiveness:u32,price:BalanceOf<T>){
			ensure_root(origin.clone())?;
			let account_id = ensure_signed(origin)?;
			let weaponry_id = Self::next_weaponry_id()?;
			let weaponry = Weaponry{
				name,
				ce:combat_effectiveness,
				price,
				kind
			};
			<Weaponrys<T>>::insert(weaponry_id,weaponry);
			WeaponrysCount::put(weaponry_id.checked_add(1).ok_or("overflow")?);
			Self::deposit_event(RawEvent::AddWeaponry(account_id,weaponry_id));
		}
		//购买装备
		pub fn buy_weaponry(origin,kitty_id:T::KittyIndex,weaponry_id:WeaponryIndex){
			let sender = ensure_signed(origin)?;
			ensure!(<OwnedKitties<T>>::exists((&sender, Some(kitty_id))), Error::<T>::RequiresOwner);
			if let Some(weaponry) = Self::weaponrys(weaponry_id){
				let root_key = <sudo::Module<T>>::key();
				T::Currency::transfer(&sender,&root_key, weaponry.price, ExistenceRequirement::KeepAlive)?;
				<KittyWeaponrys<T>>::insert((kitty_id,weaponry.kind),weaponry_id);
				Self::deposit_event(RawEvent::BuyWeaponry(sender,kitty_id,weaponry_id,weaponry.price));
			}
		}

		fn offchain_worker(curr_block: T::BlockNumber){
			if TryInto::<u64>::try_into(curr_block).ok().unwrap() % 5 == 0 {
				Self::do_offchain(curr_block)
			}
		}

		fn update_kitty_ec(origin,kitty_id:T::KittyIndex,ce:u32) -> dispatch_result{
			ensure_none(origin)?;
			if !Self::kitties(kitty_id).is_some(){
				return Ok(())
			}
			let mut kitty_attr = Self::kitty_attrs(kitty_id);
			if (kitty_attr.ce == u32::max_value()){
				print("overflow");
			}else{
				kitty_attr.ce = kitty_attr.ce+ce;
				<KittyAttrs<T>>::insert(kitty_id,kitty_attr);
				Self::deposit_event(RawEvent::Lottery(kitty_id));
			}
			print("submit offchain");
			Ok(())
		}
	}
}

fn combine_dna(dna1: u8, dna2: u8, selector: u8) -> u8 {
	((selector & dna1) | (!selector & dna2))
}

impl<T: Trait> Module<T> {
	//判断猫是否处于战斗中，繁殖、再挑战、打野、回血都需调用
	fn check_battling(kitty_id:T::KittyIndex)->bool{
		// let now = <timestamp::Module<T>>::get();
		let curr_block = <system::Module<T>>::block_number();
		let kitty_attrs = Self::kitty_attrs(kitty_id);
		if(!kitty_attrs.battle_begin.is_some()){
			return false;
		}
		if(kitty_attrs.battle_end.is_some() && kitty_attrs.battle_end.unwrap() < curr_block){
			return false
		}
		true
	}

	//取猫攻击力
	fn get_kitty_ce(kitty_id:&T::KittyIndex) -> u32 {
		let attrs = Self::kitty_attrs(kitty_id);
		let mut weapon_ce = 0u32;
		weapon_ce += Self::get_kitty_weaponry_ce(kitty_id,WeaponryKind::HELMET);
		weapon_ce += Self::get_kitty_weaponry_ce(kitty_id,WeaponryKind::ARMOR);
		weapon_ce += Self::get_kitty_weaponry_ce(kitty_id,WeaponryKind::WEAPON);
		weapon_ce += Self::get_kitty_weaponry_ce(kitty_id,WeaponryKind::SHOES);
		print("get_kitty_ce");
		print(weapon_ce);
		attrs.ce+weapon_ce
	}

	fn get_kitty_weaponry_ce(kitty_id:&T::KittyIndex,kind:WeaponryKind)->u32{
		if let Some(weapon_id) = Self::kitty_weaponrys((kitty_id,kind)){
			if let Some(weapon) = Self::weaponrys(weapon_id){
				return weapon.ce;
			}
		}
		return 0;
	}

	fn random_value(sender: &T::AccountId) -> [u8; 16] {
		let payload = (
			T::Randomness::random_seed(),
			&sender,
			<system::Module<T>>::extrinsic_index(),
			<system::Module<T>>::block_number(),
		);
		payload.using_encoded(blake2_128)
	}

	fn next_kitty_id() -> result::Result<T::KittyIndex, DispatchError> {
		let kitty_id = Self::kitties_count();
		if kitty_id == T::KittyIndex::max_value() {
			return Err(Error::<T>::KittiesCountOverflow.into());
		}
		Ok(kitty_id)
	}

	fn next_weaponry_id()-> result::Result<WeaponryIndex, DispatchError> {
		let weaponry_id = Self::weaponrys_count();
		if weaponry_id == WeaponryIndex::max_value(){
			return Err(Error::<T>::IndexCountOverflow.into());
		}
		Ok(weaponry_id)
	}

	fn insert_owned_kitty(owner: &T::AccountId, kitty_id: T::KittyIndex) {
		<OwnedKittiesList<T>>::append(owner, kitty_id);
	}

	fn insert_kitty(owner: &T::AccountId, kitty_id: T::KittyIndex, kitty: Kitty) {
		// Create and store kitty
		<Kitties<T>>::insert(kitty_id, kitty);
		<KittiesCount<T>>::put(kitty_id + 1.into());
		<KittyOwners<T>>::insert(kitty_id, owner.clone());
		//TODO 添加战斗属性默认值
		<KittyAttrs<T>>::insert(kitty_id,KittyAttr{
			hp:100, 
			exp:0,
			ce:100,
			battle_begin: None,
			battle_end: None,
			battle_type: None,
		});
		Self::insert_owned_kitty(owner, kitty_id);
	}

	fn do_breed(sender: &T::AccountId, kitty_id_1: T::KittyIndex, kitty_id_2: T::KittyIndex) -> result::Result<T::KittyIndex, DispatchError> {
		let kitty1 = Self::kitties(kitty_id_1);
		let kitty2 = Self::kitties(kitty_id_2);

		ensure!(kitty1.is_some(), Error::<T>::InvalidKittyId);
		ensure!(kitty2.is_some(), Error::<T>::InvalidKittyId);
		ensure!(kitty_id_1 != kitty_id_2, Error::<T>::RequiresDifferentParents);
		ensure!(Self::kitty_owner(&kitty_id_1).map(|owner| owner == *sender).unwrap_or(false), Error::<T>::RequiresOwner);
 		ensure!(Self::kitty_owner(&kitty_id_2).map(|owner| owner == *sender).unwrap_or(false), Error::<T>::RequiresOwner);

		let kitty_id = Self::next_kitty_id()?;

		let kitty1_dna = kitty1.unwrap().0;
		let kitty2_dna = kitty2.unwrap().0;

		// Generate a random 128bit value
		let selector = Self::random_value(&sender);
		let mut new_dna = [0u8; 16];

		// Combine parents and selector to create new kitty
		for i in 0..kitty1_dna.len() {
			new_dna[i] = combine_dna(kitty1_dna[i], kitty2_dna[i], selector[i]);
		}

		Self::insert_kitty(sender, kitty_id, Kitty(new_dna));

		Ok(kitty_id)
	}

	fn do_transfer(from: &T::AccountId, to: &T::AccountId, kitty_id: T::KittyIndex)  {
 		<OwnedKittiesList<T>>::remove(&from, kitty_id);
 		<OwnedKittiesList<T>>::append(&to, kitty_id);
 		<KittyOwners<T>>::insert(kitty_id, to);
	 }
	
	//通过offchainworker随机给链上猫侠攻击力
	 pub(crate) fn do_offchain(curr_block:T::BlockNumber){
		let kitty_count = Self::kitties_count();
		if(kitty_count==0.into()){
			print("There is nothing");
			return
		}
		let payload = (
			T::Randomness::random_seed(),
			<system::Module<T>>::extrinsic_index(),
			<system::Module<T>>::block_number(),
		);
		let random_seed  = payload.using_encoded(blake2_128);
		let now = <timestamp::Module<T>>::get();
		let random_seed = BlakeTwo256::hash(&random_seed);
		let mut rng = <RandomNumberGenerator<BlakeTwo256>>::new(random_seed);
		let random = rng.pick_u32(kitty_count.into()-1);
		print(random);
		let kitty_id:T::KittyIndex = random.into();
		print("random kitty index");
		let call = Call::update_kitty_ec(kitty_id,1u32);
		let result = T::SubmitTransaction::submit_unsigned(call);
		match result{
			Ok(_)=>{
				print("success")
			},
			Err(_)=>{
				()
				// print("offchain submit error")
			}
		}
		print("Congratulations");
	}
}


impl<T: Trait> frame_support::unsigned::ValidateUnsigned for Module<T> {
	type Call = Call<T>;

	fn validate_unsigned(call: &Self::Call) -> TransactionValidity {
		match call {
			Call::update_kitty_ec(_,_) => Ok(ValidTransaction {
				priority: 0,
				requires: vec![],
				provides: vec![0.encode()],
				longevity: TransactionLongevity::max_value(),
				propagate: true,
			}),
			_ => UnknownTransaction::NoUnsignedValidator.into(),
		}
	}
}

/// Tests for Kitties module
#[cfg(test)]
mod tests {
	use super::*;

	use sp_core::H256;
	use frame_support::{impl_outer_origin, assert_ok, parameter_types, weights::Weight};
	use sp_runtime::{
		traits::{BlakeTwo256, IdentityLookup}, testing::Header, Perbill,
	};

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	// For testing the module, we construct most of a mock runtime. This means
	// first constructing a configuration type (`Test`) which `impl`s each of the
	// configuration traits of modules we want to use.
	#[derive(Clone, Eq, PartialEq, Debug)]
	pub struct Test;
	parameter_types! {
		pub const BlockHashCount: u64 = 250;
		pub const MaximumBlockWeight: Weight = 1024;
		pub const MaximumBlockLength: u32 = 2 * 1024;
		pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
	}
	impl system::Trait for Test {
		type Origin = Origin;
		// type Call = ();
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = ();
		type BlockHashCount = BlockHashCount;
		type MaximumBlockWeight = MaximumBlockWeight;
		type MaximumBlockLength = MaximumBlockLength;
		type AvailableBlockRatio = AvailableBlockRatio;
		type Version = ();
		type ModuleToIndex = ();
	}
	parameter_types! {
		pub const ExistentialDeposit: u64 = 0;
		pub const TransferFee: u64 = 0;
		pub const CreationFee: u64 = 0;
	}
	impl balances::Trait for Test {
		type Balance = u64;
		type OnFreeBalanceZero = ();
		type OnNewAccount = ();
		type OnReapAccount = ();
		type Event = ();
		type TransferPayment = ();
		type DustRemoval = ();
		type ExistentialDeposit = ExistentialDeposit;
		type TransferFee = TransferFee;
		type CreationFee = CreationFee;
	}
	impl Trait for Test {
		type KittyIndex = u32;
		type Currency = balances::Module<Test>;
		type Randomness = randomness_collective_flip::Module<Test>;
		type Event = ();
	}
	type OwnedKittiesTest = OwnedKitties<Test>;

	// This function basically just builds a genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext() -> sp_io::TestExternalities {
		system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
	}

	#[test]
	fn owned_kitties_can_append_values() {
		new_test_ext().execute_with(|| {
			OwnedKittiesList::<Test>::append(&0, 1);

			assert_eq!(OwnedKittiesTest::get(&(0, None)), Some(KittyLinkedItem::<Test> {
				prev: Some(1),
				next: Some(1),
			}));

			assert_eq!(OwnedKittiesTest::get(&(0, Some(1))), Some(KittyLinkedItem::<Test> {
				prev: None,
				next: None,
			}));

			OwnedKittiesList::<Test>::append(&0, 2);

			assert_eq!(OwnedKittiesTest::get(&(0, None)), Some(KittyLinkedItem::<Test> {
				prev: Some(2),
				next: Some(1),
			}));

			assert_eq!(OwnedKittiesTest::get(&(0, Some(1))), Some(KittyLinkedItem::<Test> {
				prev: None,
				next: Some(2),
			}));

			assert_eq!(OwnedKittiesTest::get(&(0, Some(2))), Some(KittyLinkedItem::<Test> {
				prev: Some(1),
				next: None,
			}));

			OwnedKittiesList::<Test>::append(&0, 3);

			assert_eq!(OwnedKittiesTest::get(&(0, None)), Some(KittyLinkedItem::<Test> {
				prev: Some(3),
				next: Some(1),
			}));

			assert_eq!(OwnedKittiesTest::get(&(0, Some(1))), Some(KittyLinkedItem::<Test> {
				prev: None,
				next: Some(2),
			}));

			assert_eq!(OwnedKittiesTest::get(&(0, Some(2))), Some(KittyLinkedItem::<Test> {
				prev: Some(1),
				next: Some(3),
			}));

			assert_eq!(OwnedKittiesTest::get(&(0, Some(3))), Some(KittyLinkedItem::<Test> {
				prev: Some(2),
				next: None,
			}));
		});
	}

	#[test]
	fn owned_kitties_can_remove_values() {
		new_test_ext().execute_with(|| {
			OwnedKittiesList::<Test>::append(&0, 1);
			OwnedKittiesList::<Test>::append(&0, 2);
			OwnedKittiesList::<Test>::append(&0, 3);

			OwnedKittiesList::<Test>::remove(&0, 2);

			assert_eq!(OwnedKittiesTest::get(&(0, None)), Some(KittyLinkedItem::<Test> {
				prev: Some(3),
				next: Some(1),
			}));

			assert_eq!(OwnedKittiesTest::get(&(0, Some(1))), Some(KittyLinkedItem::<Test> {
				prev: None,
				next: Some(3),
			}));

			assert_eq!(OwnedKittiesTest::get(&(0, Some(2))), None);

			assert_eq!(OwnedKittiesTest::get(&(0, Some(3))), Some(KittyLinkedItem::<Test> {
				prev: Some(1),
				next: None,
			}));

			OwnedKittiesList::<Test>::remove(&0, 1);

			assert_eq!(OwnedKittiesTest::get(&(0, None)), Some(KittyLinkedItem::<Test> {
				prev: Some(3),
				next: Some(3),
			}));

			assert_eq!(OwnedKittiesTest::get(&(0, Some(1))), None);

			assert_eq!(OwnedKittiesTest::get(&(0, Some(2))), None);

			assert_eq!(OwnedKittiesTest::get(&(0, Some(3))), Some(KittyLinkedItem::<Test> {
				prev: None,
				next: None,
			}));

			OwnedKittiesList::<Test>::remove(&0, 3);

			assert_eq!(OwnedKittiesTest::get(&(0, None)), Some(KittyLinkedItem::<Test> {
				prev: None,
				next: None,
			}));

			assert_eq!(OwnedKittiesTest::get(&(0, Some(1))), None);

			assert_eq!(OwnedKittiesTest::get(&(0, Some(2))), None);

			assert_eq!(OwnedKittiesTest::get(&(0, Some(2))), None);
		});
	}
}
