// Copyright (c) 2020-2022 Bitcoin Dev Kit Developers
//
// This file is licensed under the Apache License, Version 2.0 <LICENSE-APACHE
// or http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your option.
// You may not use this file except in accordance with one or both of these
// licenses.

use bdk::bitcoin::secp256k1::Secp256k1;
use bdk::bitcoin::util::bip32;
use bdk::bitcoin::util::bip32::{DerivationPath, IntoDerivationPath};
use bdk::bitcoin::Network::Testnet;
use bdk::bitcoin::{secp256k1, Address, Network};
use bdk::blockchain::{Blockchain, ElectrumBlockchain};
use bdk::database::SqliteDatabase;
use bdk::descriptor::DescriptorPublicKey;
use bdk::keys::{DescriptorKey, GeneratableKey, GeneratedKey, IntoDescriptorKey};
use bdk::miniscript::descriptor::{DescriptorSecretKey, DescriptorXKey, Wildcard};
use bdk::miniscript::Legacy;
use bdk::wallet::AddressIndex;
use bdk::{descriptor, SyncOptions};
use bdk::{FeeRate, SignOptions, Wallet};
use electrum_client::Client;
use log::info;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::str::FromStr;

/// This example shows how to create and sign a large PSBTs and broadcast the finalized PSBT transaction for
/// a pay to taproot (P2TR), t of n multisig script path, descriptor wallet.
/// The electrum protocol is used to sync blockchain data from the testnet bitcoin network and
/// wallet data is stored in an ephemeral in-memory database.
/// This test was inspired by the 998 of 999 tx made by Burak on Oct 9, 2022.
/// https://twitter.com/brqgoo/status/1579216353780957185
/// TODO: figure out why we get a "Descriptor(Miniscript(AnalysisError(BranchExceedResouceLimits)))" error with more than 998 total keys.
fn main() -> Result<(), Box<dyn Error>> {
    env_logger::builder().init();
    let secp = Secp256k1::new();

    // load existing keys, or create and save new keys

    const SECRET_KEYS: usize = 997;
    const PUBLIC_KEYS: usize = 1;
    const CONFIG_FILE: &str = "tabwallet.toml";
    const DB_FILE: &str = "tabwallet.db";

    let config: Config = match fs::read_to_string(CONFIG_FILE) {
        Ok(config_string) => toml::from_str(config_string.as_str()).unwrap(),
        Err(_) => {
            let bip86_path = "m/86'/1'/0'".into_derivation_path().unwrap();
            let external_path = "m/0".into_derivation_path().unwrap();

            let secret_keys = [(); SECRET_KEYS].map(|_| {
                new_secret_key(&secp, Testnet, bip86_path.clone(), external_path.clone()).unwrap()
            });
            let secret_keys_config = secret_keys.map(|sk| sk.to_string()).to_vec();
            let public_keys = [(); PUBLIC_KEYS].map(|_| {
                new_secret_key(&secp, Testnet, bip86_path.clone(), external_path.clone())
                    .unwrap()
                    .as_public(&secp)
                    .unwrap()
            });
            let public_keys_config = public_keys.map(|pk| pk.to_string()).to_vec();
            let config = Config {
                secret_keys: secret_keys_config,
                public_keys: public_keys_config,
            };
            let config_string = toml::to_string(&config).unwrap();
            fs::write(CONFIG_FILE, config_string).unwrap();
            config
        }
    };

    let secret_keys: Vec<DescriptorKey<_>> = config
        .secret_keys
        .iter()
        .map(|sk| {
            DescriptorSecretKey::from_str(sk)
                .unwrap()
                .into_descriptor_key()
                .unwrap()
        })
        .collect();
    let public_keys: Vec<DescriptorKey<_>> = config
        .public_keys
        .iter()
        .map(|pk| {
            DescriptorPublicKey::from_str(pk)
                .unwrap()
                .into_descriptor_key()
                .unwrap()
        })
        .collect();

    let mut keys = secret_keys;
    keys.extend(public_keys);

    let unspendable_key = DescriptorPublicKey::from_str(
        "020000000000000000000000000000000000000000000000000000000000000001",
    )?;

    info!(
        "total keys: {}, threshold/signing keys: {}",
        &keys.len(),
        SECRET_KEYS
    );

    let descriptor =
        descriptor!(tr(unspendable_key.clone(), multi_a_vec(SECRET_KEYS, keys))).unwrap();
    //info!("descriptor: {}", &descriptor.0);

    // create client for Blockstream's testnet electrum server
    let blockchain =
        ElectrumBlockchain::from(Client::new("ssl://electrum.blockstream.info:60002")?);

    // create sqlite db to store cached blockchain data
    let database = SqliteDatabase::new(DB_FILE);

    // create shared watch only wallet
    let wallet: Wallet<SqliteDatabase> = Wallet::new(descriptor, None, Network::Testnet, database)?;

    info!("Syncing wallet.");
    wallet.sync(&blockchain, SyncOptions::default())?;

    // get deposit address
    let deposit_address = wallet.get_address(AddressIndex::New)?;

    let balance = wallet.get_balance()?;
    info!("Wallet balances in SATs: {}", balance);

    if balance.get_total() < 1000 {
        info!(
            "Send at least 10000 SATs (0.0001 BTC) from the u01.net testnet faucet to address '{addr}'.\nFaucet URL: https://bitcoinfaucet.uo1.net/?to={addr}",
            addr = deposit_address.address
        );
    } else if balance.get_spendable() < 10000 {
        info!(
            "Wait for at least 10000 SATs of your wallet transactions to be confirmed...\nBe patient, this could take 10 mins or longer depending on how testnet is behaving."
        );
        for tx_details in wallet
            .list_transactions(false)?
            .iter()
            .filter(|txd| txd.received > 0 && txd.confirmation_time.is_none())
        {
            info!(
                "See unconfirmed tx for {} SATs: https://mempool.space/testnet/tx/{}",
                tx_details.received, tx_details.txid
            );
        }
    } else {
        info!("Creating a PSBT sending all SATs plus fee back to the u01.net testnet faucet return address 'tb1ql7w62elx9ucw4pj5lgw4l028hmuw80sndtntxt'.");
        let return_address = Address::from_str("tb1ql7w62elx9ucw4pj5lgw4l028hmuw80sndtntxt")?;
        let mut builder = wallet.build_tx();
        builder
            .drain_to(return_address.script_pubkey())
            .enable_rbf()
            .add_data(format!("Almost a Burak tx, made with 🧡 by BDK").as_bytes())
            .fee_rate(FeeRate::from_sat_per_vb(1.25));

        let (mut psbt, details) = builder.finish()?;
        info!("Transaction details: {:#?}", details);
        info!("Unsigned PSBT: {}", psbt);

        // Sign and finalize the PSBT with the signing wallet
        let finalized = wallet.sign(&mut psbt, SignOptions::default())?;
        assert!(finalized, "The PSBT was not finalized!");
        info!("The PSBT has been signed and finalized.");

        // Broadcast the transaction
        let raw_transaction = psbt.extract_tx();
        let txid = raw_transaction.txid();
        let txurl = format!("https://mempool.space/testnet/tx/{}", &txid);

        blockchain.broadcast(&raw_transaction)?;
        info!("Transaction broadcast! TXID: {}.", txid);
        info!("Explorer URL: {}", txurl);
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    secret_keys: Vec<String>,
    public_keys: Vec<String>,
}

fn new_secret_key<C: secp256k1::Signing>(
    secp: &Secp256k1<C>,
    network: Network,
    derive_path: DerivationPath,
    extend_path: DerivationPath,
) -> Result<DescriptorSecretKey, Box<dyn Error>> {
    let origin_xprv: GeneratedKey<bip32::ExtendedPrivKey, Legacy> =
        bip32::ExtendedPrivKey::generate(())?;
    let mut derived_xprv = origin_xprv.derive_priv(&secp, &derive_path)?;
    derived_xprv.network = network;
    let key_source = (origin_xprv.fingerprint(&secp), derive_path);
    Ok(DescriptorSecretKey::XPrv(DescriptorXKey {
        origin: Some(key_source),
        xkey: derived_xprv,
        derivation_path: extend_path,
        wildcard: Wildcard::Unhardened,
    }))
}
