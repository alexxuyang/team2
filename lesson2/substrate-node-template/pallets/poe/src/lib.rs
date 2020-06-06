#![cfg_attr(not(feature = "std"), no_std)]

/// A FRAME pallet template with necessary imports

/// Feel free to remove or edit this file as needed.
/// If you change the name of this file, make sure to update its references in runtime/src/lib.rs
/// If you remove this file, you can remove those references

/// For more guidance on Substrate FRAME, see the example pallet
/// https://github.com/paritytech/substrate/blob/master/frame/example/src/lib.rs

/// 模板引入依赖
use frame_support::{decl_module, decl_storage, decl_event, decl_error, dispatch, ensure};
use frame_system::{self as system, ensure_signed};
use sp_std::prelude::*;  // 使用了Vec
use sp_runtime::traits::StaticLookup;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

/// 主程序
/// The pallet's configuration trait.
pub trait Trait: system::Trait {
	// Add other types and constants required to configure this pallet.

	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

// 存储单元
// This pallet's storage items.
decl_storage! {
	// It is important to update your storage name so that your pallet's
	// storage items are isolated from other pallets.
	// ---------------------------------vvvvvvvvvvvvvv
	trait Store for Module<T: Trait> as PoeModule {
		// 定义存储单元，用于存储存证归属信息
		// 定义一个存储项 Proofs ，给它一个default get函数，称之为proofs
		// 类型给一个map，它的key是Vec<u8>，即存证hash值，由于无法得知使用哪些hash函数，所有使用变长类型u8
		// 存证归属信息需要归属到某一个人身上，以及它在哪个时间点被存储的。这里使用tuple，给两个参数，一个是用户信息（AccountId），一个是区块链时间（BlockNumber）
		// 由于Vec<u8>是由用户输入，属于非安全，这里得使用blake2_128_concat
		// 一个简单存储单元编码完成，可用 cargo check 来检查语法是否错误
		Proofs get(fn proofs): map hasher(blake2_128_concat) Vec<u8> => (T::AccountId,T::BlockNumber)
	}
}

// 事件
// The pallet's events
decl_event!(
	pub enum Event<T> where AccountId = <T as system::Trait>::AccountId {
		ClaimCreated(AccountId,Vec<u8>),  // 用户AccountId，存证内容 Vec<u8>
		ClaimRevoked(AccountId,Vec<u8>),
	}
);

// 异常处理
// The pallet's errors
decl_error! {
	pub enum Error for Module<T: Trait> {
		NoneValue,
		StorageOverflow,
		ProofAlreadyExist,    // 存证已经存在
		ClaimNotExist,
		NotClaimOwner,
	}
}


// 可调用函数
// The pallet's dispatchable functions.
decl_module! {
	/// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing errors
		// this includes information about your errors in the node's metadata.
		// it is needed only if you are using errors in your pallet
		type Error = Error<T>;

		// Initializing events
		// this is needed only if you are using events in your pallet
		fn deposit_event() = default;

		// 创建存证，创建存证需要有两个关键参数：交易发送方origin，存证hash值claim，由于存证hash函数未知，也和decl_storage定义对应，这里使用变长Vec<u8>
        #[weight = 0]
		pub fn create_claim(origin,claim:Vec<u8>)->dispatch::DispatchResult{
			// 做必要检查，检查内容： 1，交易发送方是不是一个签名的用户 2，存证是否被别人创建过，创建过就抛出错误
			// 首先去创建签名交易，通过ensure_signed这样的system提供的版本方法来校验
			let sender = ensure_signed(origin)?;  // 存证拥有人是交易发送方，只有拥有人才可以调用存证，sender即当前交易发送方
  			// 如果存在存证，返回错误 ProofAlreadyExist
  			// ps:ensure!宏是确保表达式中的结果为true，这里取反操作
			ensure!(!Proofs::<T>::contains_key(&claim),Error::<T>::ProofAlreadyExist);  // 这里用到一个错误  ProofAlreadyExist，该错误需要在decl_error声明
			// 做insert操作，insert是key-value方式。这里的key-value是一个tuple
			// 这个tuple的第一个元素是AccountId；第二个是当前交易所处的区块，使用系统模块提供的block_number工具方法获取
			Proofs::<T>::insert(&claim,(sender.clone(),system::Module::<T>::block_number()));  // 插入操作
			// 触发一个event来通知客户端，RawEvent由宏生成；   sender:存在拥有人；claim:存在hash值 通过event通知客户端
			Self::deposit_event(RawEvent::ClaimCreated(sender,claim));
			// 返回ok
			Ok(())

		}

        #[weight = 0]
		pub fn revoke_claim(origin,claim: Vec<u8>) -> dispatch::DispatchResult{
			let sender = ensure_signed(origin)?;  // 交易发送方式已签名的， 存证拥有人是交易发送方，只有拥有人才可以吊销存证

  			// 判断存储单元里面是存在这样一个存证；如果不存在，抛出错误，错误我们叫ClaimNotExist
			ensure!(Proofs::<T>::contains_key(&claim),Error::<T>::ClaimNotExist);

			// 获取这样的存证  owner: accountId   block_number
			let (owner,_block_number) = Proofs::<T>::get(&claim);  // 通过get api获取这样的一个存证

			ensure!(owner == sender,Error::<T>::NotClaimOwner);  // 确保交易发送方是我们的存证人，如果不是，返回Error，这个Error我们叫NotClaimOwner

			// 以上校验完成之后，我们就可以删除我们的存证
		    // 存储向上调用remove函数进行删除
		    Proofs::<T>::remove(&claim);

			// 触发一个事件，返回存证人和hash
		    Self::deposit_event(RawEvent::ClaimRevoked(sender,claim));

			// 返回
			Ok(())

		}


		// 转移存证
	    #[weight=0]
        pub fn transfer_claim(origin,claim:Vec<u8>,dest:<T::Lookup as StaticLookup>::Source)->dispatch::DispatchResult{
            let sender=ensure_signed(origin)?;
            ensure!(Proofs::<T>::contains_key(&claim),Error::<T>::ClaimNotExist);
            let(owner,_block_number)=Proofs::<T>::get(&claim);
            ensure!(owner==sender,Error::<T>::NotClaimOwner);
            let dest=T::Lookup::lookup(dest)?;
            Proofs::<T>::insert(&claim,(dest,system::Module::<T>::block_number()));
            Ok(())
        }


	}
}

