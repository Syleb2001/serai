use super::*;

use scale::{Encode, Decode};

use serai_primitives::{
  Amount, Balance, Coin, ExternalAddress, ExternalBalance, ExternalCoin, ExternalNetworkId,
  NetworkId, SeraiAddress, BlockHash,
};

use coins_primitives::OutInstruction;

/// Assert that a value SCALE-encodes and decodes back to itself, consuming exactly the encoded
/// bytes. These encodings are consensus-critical (e.g. signed `Batch`s) and must remain stable.
fn scale_round_trip<T: Encode + Decode + PartialEq + core::fmt::Debug>(value: T) {
  let encoded = value.encode();
  let mut slice = encoded.as_slice();
  let decoded = T::decode(&mut slice).expect("decoding failed");
  assert_eq!(decoded, value);
  assert!(slice.is_empty(), "decoding did not consume all bytes");
}

#[test]
fn in_instruction_variants_round_trip() {
  let addr = SeraiAddress([0x11; 32]);

  scale_round_trip(InInstruction::Transfer(addr));
  scale_round_trip(InInstruction::Dex(DexCall::SwapAndAddLiquidity(addr)));
  scale_round_trip(InInstruction::Dex(DexCall::Swap(
    Balance { coin: Coin::Serai, amount: Amount(123) },
    OutAddress::Serai(addr),
  )));
  scale_round_trip(InInstruction::Dex(DexCall::Swap(
    Balance { coin: Coin::External(ExternalCoin::Bitcoin), amount: Amount(7) },
    OutAddress::External(ExternalAddress::new(vec![1, 2, 3]).unwrap()),
  )));
  scale_round_trip(InInstruction::GenesisLiquidity(addr));
  scale_round_trip(InInstruction::SwapToStakedSRI(
    addr,
    NetworkId::External(ExternalNetworkId::Ethereum),
  ));
}

#[test]
fn refundable_in_instruction_round_trips_with_and_without_origin() {
  let addr = SeraiAddress([0x22; 32]);

  scale_round_trip(RefundableInInstruction {
    origin: None,
    instruction: InInstruction::Transfer(addr),
  });
  scale_round_trip(RefundableInInstruction {
    origin: Some(ExternalAddress::new(vec![9, 9, 9]).unwrap()),
    instruction: InInstruction::Transfer(addr),
  });
}

#[test]
fn shorthand_variants_round_trip() {
  let addr = SeraiAddress([0x33; 32]);

  scale_round_trip(Shorthand::transfer(None, addr));
  scale_round_trip(Shorthand::Swap {
    origin: Some(ExternalAddress::new(vec![1]).unwrap()),
    coin: ExternalCoin::Bitcoin,
    minimum: Amount(10),
    out: OutInstruction { address: ExternalAddress::new(vec![2, 3]).unwrap(), data: None },
  });
  scale_round_trip(Shorthand::SwapAndAddLiquidity {
    origin: None,
    minimum: Amount(5),
    gas: Amount(1),
    address: addr,
  });
}

#[test]
fn shorthand_transfer_builds_raw_transfer() {
  let addr = SeraiAddress([0x44; 32]);

  match Shorthand::transfer(None, addr) {
    Shorthand::Raw(RefundableInInstruction {
      origin: None,
      instruction: InInstruction::Transfer(a),
    }) => assert_eq!(a, addr),
    _ => panic!("Shorthand::transfer did not build a Raw Transfer"),
  }

  // A Raw shorthand converts back into its inner instruction.
  let inner =
    RefundableInInstruction { origin: None, instruction: InInstruction::Transfer(addr) };
  let converted: RefundableInInstruction = Shorthand::Raw(inner.clone()).try_into().unwrap();
  assert_eq!(converted, inner);
}

#[test]
fn batch_round_trips_and_message_is_prefixed() {
  let batch = Batch {
    network: ExternalNetworkId::Bitcoin,
    id: 42,
    block: BlockHash([0x55; 32]),
    instructions: vec![
      InInstructionWithBalance {
        instruction: InInstruction::Transfer(SeraiAddress([1; 32])),
        balance: ExternalBalance { coin: ExternalCoin::Bitcoin, amount: Amount(1000) },
      },
      InInstructionWithBalance {
        instruction: InInstruction::GenesisLiquidity(SeraiAddress([2; 32])),
        balance: ExternalBalance { coin: ExternalCoin::Monero, amount: Amount(5) },
      },
    ],
  };
  scale_round_trip(batch.clone());

  // `batch_message` is the batch's SCALE encoding prefixed with a domain-separating tag.
  let message = batch_message(&batch);
  assert!(message.starts_with(b"InInstructions-batch"));
  assert_eq!(message, [b"InInstructions-batch".as_ref(), &batch.encode()].concat());
}

#[test]
fn out_address_helpers() {
  let native = OutAddress::Serai(SeraiAddress([7; 32]));
  assert!(native.is_native());
  assert_eq!(native.clone().as_native(), Some(SeraiAddress([7; 32])));
  assert_eq!(native.as_external(), None);

  let ext_addr = ExternalAddress::new(vec![8, 8]).unwrap();
  let external = OutAddress::External(ext_addr.clone());
  assert!(!external.is_native());
  assert_eq!(external.clone().as_native(), None);
  assert_eq!(external.as_external(), Some(ext_addr));
}
