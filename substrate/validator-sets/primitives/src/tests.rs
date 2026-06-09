use super::*;

fn public(byte: u8) -> Public {
  Public::from([byte; 32])
}

#[test]
fn musig_context_is_domain_separated_and_deterministic() {
  let set = ValidatorSet { session: Session(5), network: NetworkId::Serai };

  let context = musig_context(set);
  // Prefixed with the domain-separating tag.
  assert!(context.starts_with(b"ValidatorSets-musig_key"));
  // Deterministic.
  assert_eq!(context, musig_context(set));

  // Distinct sets yield distinct contexts.
  let other = ValidatorSet { session: Session(6), network: NetworkId::Serai };
  assert_ne!(musig_context(set), musig_context(other));
  let other_network =
    ValidatorSet { session: Session(5), network: NetworkId::External(ExternalNetworkId::Bitcoin) };
  assert_ne!(musig_context(set), musig_context(other_network));
}

#[test]
fn amortize_is_a_noop_at_or_below_the_maximum() {
  // Total of 3 + 3 = 6 key shares, well below MAX_KEY_SHARES_PER_SET.
  let mut validators = [(public(1), 3u64), (public(2), 3u64)];
  let before = validators;
  amortize_excess_key_shares(&mut validators);
  assert_eq!(validators, before);
}

#[test]
fn amortize_reduces_to_exactly_the_maximum_reverse_round_robin() {
  // Total of 201 key shares; excess of 51 over the maximum of 150.
  let mut validators = [(public(1), 101u64), (public(2), 100u64)];
  amortize_excess_key_shares(&mut validators);

  let total: u64 = validators.iter().map(|(_, shares)| *shares).sum();
  assert_eq!(total, u64::from(MAX_KEY_SHARES_PER_SET));

  // Reduction is a reverse round-robin, so the last validator is reduced first (and thus more):
  // 51 decrements over 2 validators => last gets 26, first gets 25.
  assert_eq!(validators, [(public(1), 76u64), (public(2), 74u64)]);
}

#[test]
fn post_amortization_shares_for_top_validator() {
  // No excess => the top validator keeps all its shares.
  assert_eq!(post_amortization_key_shares_for_top_validator(5, 30, 100), 30);

  // Excess of 30 over 3 validators => the top loses 30 / 3 = 10 shares.
  assert_eq!(post_amortization_key_shares_for_top_validator(3, 100, 180), 90);
}

#[test]
fn messages_are_domain_separated_and_input_sensitive() {
  let set = ExternalValidatorSet { session: Session(1), network: ExternalNetworkId::Bitcoin };
  let key_pair = KeyPair(public(7), ExternalKey::try_from(vec![1u8, 2, 3]).unwrap());

  let keys_msg = set_keys_message(&set, &[], &key_pair);
  assert!(keys_msg.starts_with(b"ValidatorSets-set_keys"));
  assert_eq!(keys_msg, set_keys_message(&set, &[], &key_pair));

  // A different removed-participants list changes the message.
  let keys_msg_removed = set_keys_message(&set, &[public(9)], &key_pair);
  assert_ne!(keys_msg, keys_msg_removed);

  let slashes = [(public(3), 10u32), (public(4), 5u32)];
  let slash_msg = report_slashes_message(&set, &slashes);
  assert!(slash_msg.starts_with(b"ValidatorSets-report_slashes"));
  assert_eq!(slash_msg, report_slashes_message(&set, &slashes));

  // Different slash amounts change the message; the two tags never collide.
  assert_ne!(slash_msg, report_slashes_message(&set, &[(public(3), 11u32), (public(4), 5u32)]));
  assert_ne!(keys_msg, slash_msg);
}
