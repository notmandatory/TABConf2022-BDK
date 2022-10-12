# TABConf 2022

## BDK - Bitcoin Dev Kit, Builder Day
October 12, 10:30am - 4:45pm

### Beginner level

#### Prerequisites

* Laptop that runs Linux or MacOS.
    * Windows may work but is not recommended.
* Be comfortable using the command line.
* Know how to install new software (ie. apt, brew).

#### Reference Docs

* Rust, [The Cargo Book](https://doc.rust-lang.org/cargo/)
* BDK docs, [BDK CLI Introduction](https://bitcoindevkit.org/bdk-cli/introduction/)
* GitHub, [About Git](https://docs.github.com/en/get-started/using-git/about-git)
* Sipa on [Miniscript](https://bitcoin.sipa.be/miniscript/)
* BIP-44 ["Multi-Account Hierarchy for Deterministic Wallets"](https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki)
* BIP-86 ["Key Derivation for Single Key P2TR Outputs"](https://github.com/bitcoin/bips/blob/master/bip-0086.mediawiki)
* BIP-360 ["Output Script Descriptors General Operation"](https://github.com/bitcoin/bips/blob/master/bip-0380.mediawiki)
* BIP-386 ["tr() Output Script Descriptors"](https://github.com/bitcoin/bips/blob/master/bip-0386.mediawiki)

#### Setup

1. [Install Rust using rustup](https://www.rust-lang.org/tools/install).
2. Verify rust is installed: 
   ```shell
   rustup show
   ```
3. Install `git`
   ```shell
   # on linux typically apt
   apt install git
   
   # on macOS brew is a good way
   brew install git
   ```
4. Verify git is installed:
   ```shell
   git --version
   ```
5. Clone and install `bdk-cli`:
   ```shell
   cd <wherever you keep your git projects>
   git clone https://github.com/bitcoindevkit/bdk-cli
   cd bdk-cli
   cargo install --path . --features electrum,verify,compiler
   bdk-cli help
   ```
6. Install `jq` tool to parse json results:
   ```shell
   # on linux typically apt
   apt install jq
   
   # on macOS brew is a good way
   brew install jq
   ```
7. Verify `jq` was installed:
   ```shell
   jq --version
   ```
8. Create your master and derived extended private keys:
   ```shell
   bdk-cli key generate > alice-key.json
   ALICE_WORDS=`cat alice-key.json | jq ".mnemonic"`
   ALICE_MASTER_XPRV=`cat alice-key.json | jq ".xprv" | tr -d '"'`
   ALICE_BIP86_XPRV=`bdk-cli key derive --path "m/86'/1'/0'" --xprv $ALICE_MASTER_XPRV | jq ".xprv" | tr -d '"*'`
   ALICE_BIP86_XPUB=`bdk-cli key derive --path "m/86'/1'/0'" --xprv $ALICE_MASTER_XPRV | jq ".xpub" | tr -d '"*'`
   UNSPENDABLE_KEY=020000000000000000000000000000000000000000000000000000000000000001
   ```
9. Create shared public key P2TR descriptor:
   ```shell
   SHARED_EXT_DESC="tr($UNSPENDABLE_KEY,multi_a(2,${ALICE_BIP86_XPUB:0:-1}/0/*,${BOB_BIP86_XPUB:0:-1}/0/*,${CAROL_BIP86_XPUB:0:-1}/0/*))"
   ```
10. Create Alice's private key P2TR descriptor:
    ```shell
    ALICE_EXT_DESC="tr($UNSPENDABLE_KEY,multi_a(2,${ALICE_BIP86_XPRV:0:-1}/0/*,${BOB_BIP86_XPUB:0:-1}/0/*,${CAROL_BIP86_XPUB:0:-1}/0/*))"
    ```
11. Sync shared wallet and get new receive address:
   ```shell
   bdk-cli wallet -d $SHARED_EXT_DESC sync
   bdk-cli wallet -d $SHARED_EXT_DESC get_new_address
   ```
12. Send testnet bitcoin from [faucet](https://bitcoinfaucet.uo1.net/) to new address.
13. Verify transaction in [mempool](https://mempool.space/testnet).
14. Verify untrusted pending balance with wallet:
   ```shell
   bdk-cli wallet -d $SHARED_EXT_DESC sync
   bdk-cli wallet -d $SHARED_EXT_DESC get_balance
   ```
15. Wait for transaction confirmation.
16. Verify confirmed balance with wallet:
   ```shell
   bdk-cli wallet -d $SHARED_EXT_DESC sync
   bdk-cli wallet -d $SHARED_EXT_DESC get_balance
   ```
17. Create unsigned PSBT for spending transaction with op_return:
    ```shell
    MESSAGE="TABConf2022 alice, bob, carol"
    UNSIGNED_PSBT=`bdk-cli wallet -d $SHARED_EXT_DESC create_tx --enable_rbf --fee_rate 2 --send_all --add_string $MESSAGE --to "tb1ql7w62elx9ucw4pj5lgw4l028hmuw80sndtntxt:0" | jq ".psbt" | tr -d '"'`
    ```
18. Alice, Bob, and Carol create PSBTs with their signatures:
    ```shell
    bdk-cli wallet -d $SHARED_EXT_DESC sync
    ALICE_SIGNED_PSBT=`bdk-cli wallet -d $ALICE_EXT_DESC sign --psbt $UNSIGNED_PSBT | jq ".psbt" | tr -d '"'`
    BOB_SIGNED_PSBT=`bdk-cli wallet -d $BOB_EXT_DESC sign --psbt $UNSIGNED_PSBT | jq ".psbt" | tr -d '"'`
    CAROL_SIGNED_PSBT=`bdk-cli wallet -d $CAROL_EXT_DESC sign --psbt $UNSIGNED_PSBT | jq ".psbt" | tr -d '"'`
    ```
19. Combine signed PSBTs and broadcast finalized transaction
   ```shell
   SIGNED_PSBT=`bdk-cli wallet -d $SHARED_EXT_DESC combine_psbt --psbt $ALICE_SIGNED_PSBT --psbt $BOB_SIGNED_PSBT --psbt $CAROL_SIGNED_PSBT | jq ".psbt" | tr -d '"'`
   FINALIZED_PSBT=`bdk-cli wallet -d $SHARED_EXT_DESC finalize_psbt --psbt $SIGNED_PSBT | jq ".psbt" | tr -d '"'`
   bdk-cli wallet -d $SHARED_EXT_DESC broadcast --psbt $FINALIZED_PSBT
   ```

### Future Work

* ["MuSig2: Simple Two-Round Schnorr Multi-Signatures”, Nick, Ruffing, and Seurin"](https://eprint.iacr.org/2020/1261)
* [BIP-??? MuSig2](https://github.com/jonasnick/bips/blob/musig2/bip-musig2.mediawiki)
* An easy way to create or use default unspendable key spend key?
   