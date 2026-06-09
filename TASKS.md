# TASKS — Achever Serai (développement test-driven)

Liste de tâches pour finaliser le fork, ordonnée par dépendances et valeur.
**Règle d'or : aucune tâche n'est « terminée » tant que son test unitaire
associé ne passe pas en vert.**

Issue de l'audit de complétude (voir l'historique de la session). Chaque tâche
indique : fichiers, ce qu'il faut faire, et **le test qui la valide**.

---

## 0. Pré-requis environnement (à refaire à chaque session web)

```bash
apt-get install -y protobuf-compiler
cargo install svm-rs && svm install 0.8.26 && svm use 0.8.26
ln -sf /usr/lib/llvm-18/lib/libclang.so.1 /usr/lib/llvm-18/lib/libclang.so
```

Build de référence (doit rester vert) :
```bash
cargo check -p serai-coordinator --features rocksdb
cargo check -p serai-processor --features "binaries bitcoin rocksdb"
```

> Idéalement automatisé via un SessionStart hook (`.claude/`).

### Contraintes de l'environnement web (constatées en session)
- ✅ crates.io, `apt`, et les deps git (`serai-dex/patch-polkadot-sdk`) sont accessibles.
- 🚫 **L'API GitHub / les binaires de release sont bloqués (403)** → `foundryup`
  échoue, donc **`anvil`/`forge` ne sont pas installables**. Les tests
  `networks/ethereum` (qui lancent `anvil`) et les tests d'intégration Docker
  **ne peuvent pas tourner ici**. Le périmètre unit-testable se limite aux crates
  Rust pures sans nœud externe (crypto, primitives, pallets légers coins/dex/
  economic-security, helpers purs). Les phases 2 (coordinator e2e) et 3 (Ethereum)
  devront être validées dans un environnement avec Docker/foundry.

---

## Phase 0 — Harnais de tests unitaires pour les pallets critiques

Les pallets `validator-sets`, `signals`, `emissions`, `genesis-liquidity`,
`economic-security` n'ont **aucun** mock runtime ni test unitaire (seulement des
tests d'intégration Docker dans `substrate/client/tests/`). On ne peut pas
valider le slashing / la sécurité économique sans ce harnais. Modèle existant :
`substrate/coins/pallet/src/{mock,tests}.rs` et `substrate/dex/pallet/src/{mock,tests}.rs`.

### T0.1 — Harnais `economic-security` (le plus simple, dépend seulement de coins+dex)
- **Fichiers :** créer `substrate/economic-security/pallet/src/mock.rs` + `tests.rs`,
  les déclarer dans `lib.rs` (`#[cfg(test)] mod mock; #[cfg(test)] mod tests;`),
  ajouter les dev-deps dans `Cargo.toml` (cf. `dex/pallet/Cargo.toml`).
- **Test de validation :** `economic_security::tests::reaches_economic_security`
  - Construit le runtime mock (System + Coins + Dex + EconomicSecurity).
  - Sans oracle/AllowMint → `economic_security_block(network) == None` après `on_initialize`.
  - Avec oracle value + mint autorisé → le bloc est enregistré **et** l'event
    `EconomicSecurityReached` est émis exactement une fois.
- **Commande :** `cargo test -p serai-economic-security-pallet`

### T0.2 — Harnais `validator-sets` (BLOQUÉ — voir note)
- **Constat (session) :** la `Config` de `validator-sets` requiert
  `pallet_session + pallet_babe + pallet_grandpa`, et ces pallets sont
  **mutuellement interdépendants** avec `validator-sets` (babe utilise
  `ValidatorSets` pour les équivocations/`DisabledValidators`, validator-sets
  utilise `Babe` pour `ShouldEndSession`/`authorities`). Construire un mock
  standalone revient à reconstruire tout le runtime → coût/fragilité
  disproportionnés. C'est pourquoi l'amont ne teste ce pallet qu'en
  **intégration** (`substrate/client/tests/validator_sets.rs`, via Docker).
- **Stratégie révisée :** tester la logique de `validator-sets` (et des pallets
  qui en dépendent : `signals`, `emissions`, `genesis-liquidity`) via
  (a) extraction de **fonctions pures** unit-testables quand on ajoute du code
  (slashing, gate de sécurité éco, seuil de halt), et (b) les **tests
  d'intégration** existants. Les tâches T1.1–T1.3 doivent donc livrer leur
  logique sous forme de helpers purs testables.

### T0.3 — Harnais `signals` / `emissions` (dépend de T0.2, même blocage)

### TS.1 — Arithmétique non-panic des primitives ✅
- **Fichiers :** `substrate/primitives/src/amount.rs`, `balance.rs`.
- **Fait :** ajout de `checked_add/sub/mul` (+ `saturating_add/sub` sur `Amount`)
  sur `Amount`, `Balance`, `ExternalBalance`, répondant aux TODO « ces impl ne
  doivent pas paniquer » sans casser l'API opérateur (additif).
- **Test :** `cargo test -p serai-primitives` (8 verts).

---

## Phase 1 — Sécurité économique & slashing (🔴 bloquant prod)

### T1.1 — Implémenter le slashing dans `validator-sets`
- **Fichier :** `substrate/validator-sets/pallet/src/lib.rs:1007` (`// TODO: Handle slashes`).
- **À faire :** appliquer la réduction d'allocation / retrait du validateur slashé,
  émettre l'event correspondant, mettre à jour `total_allocated_stake`.
- **Test :** `validator_sets::tests::slash_reduces_stake`
  - Allouer du stake à un validateur, appliquer un slash de montant X →
    son allocation diminue de X (ou il est retiré si < minimum), event émis.
- **Dépend de :** T0.2.

### T1.2 — Refuser un nouveau set sans sécurité économique
- **Fichier :** `lib.rs:376` (`// TODO: prevent new set if it doesn't have enough stake`).
- **À faire :** bloquer la promotion d'un set dont le stake total < seuil requis.
- **Test :** `validator_sets::tests::rejects_set_below_economic_security`
  - Set avec stake insuffisant → pas de rotation / erreur ; stake suffisant → OK.

### T1.3 — Seuil de halt à 34 % (et non 80 %)
- **Fichier :** `substrate/signals/pallet/src/lib.rs` (`// TODO: Use 34% for halting a set`).
- **À faire :** corriger le seuil de favorables requis pour halter un set.
- **Test :** `signals::tests::halt_requires_34_percent`
  - 33 % de favorables → set non halté ; 34 % → set halté.
- **Dépend de :** T0.3.

---

### TS.2 — Implémenter `Shorthand::{Swap, SwapAndAddLiquidity}` → `RefundableInInstruction` (⚠️ décision protocole requise)
- **Fichier :** `substrate/in-instructions/primitives/src/shorthand.rs:51-52`
  (deux `todo!()` en code de prod → **panic à l'exécution** si un tel shorthand
  est soumis).
- **Bloqueur :** ces variantes ne sont construites/décodées **nulle part** dans
  le code, et `DexCall` porte `// TODO: Update this per documentation/Shorthand`.
  Le mapping (coin = entrée ou sortie ? perte de `OutInstruction.data` ? cible
  Serai vs External ?) n'est pas spécifié → **nécessite un arbitrage de design**
  avant implémentation, sous peine d'encoder une sémantique de swap erronée.
- **Test (une fois la sémantique fixée) :** `shorthand::tests::swap_round_trips`
  vérifiant la conversion attendue + (dé)sérialisation SCALE.

## Phase 2 — Robustesse du consensus (coordinator)

### T2.1 — Reprise DKG au lieu de paniquer
- **Fichier :** `coordinator/src/tributary/scanner.rs:311`
  (`panic!("TODO: re-attempting DkgConfirmation...")`).
- **À faire :** déclencher une nouvelle tentative de DKG plutôt qu'un panic.
- **Test :** ajouter à `coordinator/src/tests/tributary/dkg.rs` un cas
  `reattempt_dkg_does_not_panic` qui force le chemin de ré-attempt.

### T2.2 — Liste réelle des participants attendus pour Sign/SubstrateSign
- **Fichier :** `scanner.rs:317` (actuellement `vec![]` en dur).
- **À faire :** dériver l'ensemble depuis les preprocesses reçus (≥ 67 %).
- **Test :** unit test du calcul d'ensemble des signataires attendus.

### T2.3 — Fermer le DoS « fatally slashed peut publier »
- **Fichier :** `coordinator/src/tributary/handle.rs:249-250`.

---

## Phase 3 — Parité de l'intégration Ethereum (🔴)

Tests existants dans `networks/ethereum/src/tests/` (nécessitent `anvil`/foundry).

### T3.1 — Définir un `DUST` Ethereum
- **Fichier :** `processor/src/networks/ethereum.rs:405` (`const DUST: u64 = 0`).
- **Test :** `ethereum::tests::dust_threshold_enforced`.

### T3.2 — Supporter les paiements DAI / ERC-20 (pas seulement Ether)
- **Fichier :** `ethereum.rs:687` (`assert_eq!(payment.balance.coin, ExternalCoin::Ether)`).
- **Test :** router test envoyant un transfert ERC-20 et vérifiant la réception.

### T3.3 — Amortissement des frais
- **Fichier :** `ethereum.rs:655-658`.
- **Test :** `ethereum::tests::fee_amortization_splits_across_outputs`.

---

## Couverture de tests (crates pures, sans infra externe)

Renforcement de la couverture unitaire sur le périmètre testable dans cet
environnement (pas de Docker/anvil).

### TS.3 — Pallet `coins` ✅
- Ajout de l'Instance1 (LiquidityTokens) au mock + tests des chemins non couverts :
  `burn` (succès/échec/event), event `Transfer` + nettoyage du storage à zéro,
  accumulation au mint, burn des fees en `on_initialize`, refus de
  `burn_with_instruction` sur les LiquidityTokens.
- **Test :** `cargo test -p serai-coins-pallet` (10 verts).

### TS.4 — `in-instructions/primitives` ✅
- Tests de round-trip d'encodage **SCALE** (consensus-critique) pour
  `InInstruction`, `RefundableInInstruction`, `Shorthand`, `Batch`, +
  `batch_message` (préfixe) et helpers `OutAddress`.
- **Test :** `cargo test -p serai-in-instructions-primitives` (6 verts).

### TS.5 — `validator-sets/primitives` ✅
- Tests des fonctions pures : `amortize_excess_key_shares` (réduction reverse
  round-robin jusqu'au max exact — consensus-critique),
  `post_amortization_key_shares_for_top_validator`, `musig_context`, et les
  messages à séparation de domaine `set_keys_message`/`report_slashes_message`.
- **Test :** `cargo test -p serai-validator-sets-primitives` (5 verts).

---

## Phase 4 — Calibration des poids (weights) — prêt mainnet

Tous les `#[pallet::weight(0)]` / `Weight::zero()` doivent être remplacés par des
poids issus du benchmarking. Modèle : `substrate/dex/pallet/{benchmarking,weights}.rs`.

### T4.x (un par pallet) — `validator-sets`, `coins`, `emissions`, `signals`, `genesis-liquidity`, `economic-security`
- **À faire :** ajouter `benchmarking.rs` + `weights.rs`, brancher `WeightInfo`.
- **Validation :** `cargo build -p serai-runtime --features runtime-benchmarks` compile,
  et les `weight(0)` ont disparu (`! grep -rn 'weight(0)' substrate/<pallet>`).

---

## Suivi

| Tâche | Statut | Test vert |
|-------|--------|-----------|
| T0.1  | ✅ fait | `cargo test -p serai-economic-security-pallet` (5 verts) |
| T0.2  | 🚫 bloqué | mock impraticable (babe/grandpa/session interdépendants) → tests d'intégration |
| T0.3  | 🚫 bloqué | dépend de T0.2 |
| TS.1  | ✅ fait | `cargo test -p serai-primitives` (8 verts) |
| TS.3  | ✅ fait | `cargo test -p serai-coins-pallet` (10 verts) |
| TS.4  | ✅ fait | `cargo test -p serai-in-instructions-primitives` (6 verts) |
| TS.5  | ✅ fait | `cargo test -p serai-validator-sets-primitives` (5 verts) |
| T1.1  | ⬜ todo | — |
| T1.2  | ⬜ todo | — |
| T1.3  | ⬜ todo | — |
| T2.1  | ⬜ todo | — |
| T2.2  | ⬜ todo | — |
| T2.3  | ⬜ todo | — |
| T3.1  | ⬜ todo | — |
| T3.2  | ⬜ todo | — |
| T3.3  | ⬜ todo | — |
| T4.x  | ⬜ todo | — |
