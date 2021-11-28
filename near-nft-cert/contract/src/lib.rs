use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::serde::{Deserialize, Serialize};

use near_contract_standards::non_fungible_token::metadata::{
    NFTContractMetadata, NonFungibleTokenMetadataProvider, TokenMetadata, NFT_METADATA_SPEC,
};
use near_contract_standards::non_fungible_token::{Token, TokenId};
use near_contract_standards::non_fungible_token::NonFungibleToken;
use near_sdk::collections::LazyOption;
use std::convert::TryFrom;
use near_sdk::json_types::ValidAccountId;
use near_sdk::{
    setup_alloc, env, near_bindgen, AccountId, BorshStorageKey, Promise, PromiseOrValue,
};

setup_alloc!();

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    NonFungibleToken,
    Metadata,
    TokenMetadata,
    Enumeration,
    Approval,
}

// DEFINE MODEL:
#[derive(BorshDeserialize, BorshSerialize, Clone, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Certificate {
    pub owner_name: String,
    pub issuer_account: ValidAccountId,
    pub is_approved: bool,
    pub metadata: TokenMetadata,
    pub owner_account: ValidAccountId 
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Issuer {
    pub name: String,
    pub account: ValidAccountId
}

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize)]
pub struct Contract {
    contract_foundation: ValidAccountId,
    issuers: UnorderedMap<ValidAccountId, Issuer>,

    certs_map: UnorderedMap<ValidAccountId, Certificate>,

    //NFT 
    nft_token: NonFungibleToken,
    metadata: LazyOption<NFTContractMetadata>,
}

impl Default for Contract {
    fn default() -> Self {
        env::panic(b"NearCert contract should be initialized before usage")
    }
}

const DATA_IMAGE_SVG_NEAR_ICON: &str = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 288 288'%3E%3Cg id='l' data-name='l'%3E%3Cpath d='M187.58,79.81l-30.1,44.69a3.2,3.2,0,0,0,4.75,4.2L191.86,103a1.2,1.2,0,0,1,2,.91v80.46a1.2,1.2,0,0,1-2.12.77L102.18,77.93A15.35,15.35,0,0,0,90.47,72.5H87.34A15.34,15.34,0,0,0,72,87.84V201.16A15.34,15.34,0,0,0,87.34,216.5h0a15.35,15.35,0,0,0,13.08-7.31l30.1-44.69a3.2,3.2,0,0,0-4.75-4.2L96.14,186a1.2,1.2,0,0,1-2-.91V104.61a1.2,1.2,0,0,1,2.12-.77l89.55,107.23a15.35,15.35,0,0,0,11.71,5.43h3.13A15.34,15.34,0,0,0,216,201.16V87.84A15.34,15.34,0,0,0,200.66,72.5h0A15.35,15.35,0,0,0,187.58,79.81Z'/%3E%3C/g%3E%3C/svg%3E";

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        assert!(!env::state_exists(), "The contract is already initialized");

        let metadata = NFTContractMetadata {
            spec: NFT_METADATA_SPEC.to_string(),
            name: "Near L1 Certificate NFT".to_string(),
            symbol: "L1".to_string(),
            icon: Some(DATA_IMAGE_SVG_NEAR_ICON.to_string()),
            base_uri: None,
            reference: None,
            reference_hash: None,
        };

        let signer = ValidAccountId::try_from(env::predecessor_account_id().clone()).unwrap();

        Contract {
            contract_foundation: signer.clone(),
            issuers: UnorderedMap::new(b"i".to_vec()),
            certs_map: UnorderedMap::new(b"cert".to_vec()),
            nft_token: NonFungibleToken::new(
                StorageKey::NonFungibleToken,
                signer,
                Some(StorageKey::TokenMetadata),
                Some(StorageKey::Enumeration),
                Some(StorageKey::Approval),
                ),
            metadata: LazyOption::new(StorageKey::Metadata, Some(&metadata)),
        }
    }

    pub fn new_issuer(&mut self, issuer: ValidAccountId, issuer_name: String) -> bool {
        self.only_owner();

        if !self.issuers.get(&issuer).is_some() {
            let _issuer = Issuer {
                name: issuer_name,
                account: issuer.clone()
            };
            self.issuers.insert(&issuer, &_issuer);
            return true;   
        }
        return false;
    }

    pub fn new_cert(
        &mut self,
        _owner_name: String,
        _owner_account: ValidAccountId, 
        _media_uri: String,
        _media_hash: String,
        ) -> Certificate {
        self.only_issuer();

        let predecessor = env::predecessor_account_id();
        let receiver_id = ValidAccountId::try_from(predecessor.clone()).unwrap();

        let creator = self.issuers.get(&receiver_id);

        let metadata = TokenMetadata {
            title: Some("L1 Certificate".into()),
            description: Some("".into()),
            media: Some(_media_uri.into()),
            media_hash: None,
            copies: Some(1u64),
            issued_at: Some(env::block_timestamp().to_string()),
            expires_at: None,
            starts_at: None,
            updated_at: None,
            extra: None,
            reference: None,
            reference_hash: None,
        };

        let cert = Certificate {
            owner_name: _owner_name,
            issuer_account: creator.unwrap().account,
            is_approved: false,
            metadata: metadata,
            owner_account: _owner_account.clone() 
        };

        self.certs_map.insert(&_owner_account, &cert);
        return cert;
    }

    // pub fn approve(&mut self, account: ValidAccountId) -> bool {
    //     assert!(
    //         self.certs_map.get(&account).is_some(),
    //         "This account doesn't have any cert"
    //         );
    //     self.only_owner();

    //     let mut cert = self.certs_map.get(&account).unwrap();
    //     cert.is_approved = true;
    //     return true;
    // }

    #[payable]
    pub fn mint_cert(&mut self, account: ValidAccountId) -> Token {
        self.only_owner();

        assert!(
            self.certs_map.get(&account).is_some(),
            "This account doesn't have any cert"
            );

        let cert = self.certs_map.get(&account).unwrap();
        let token = self.nft_token.mint(cert.owner_account.to_string(), account, Some(cert.metadata));

        return token;
    }

    #[payable]
    pub fn transfer_to_owner(&mut self, account: ValidAccountId) {
        self.only_owner();
        self.nft_transfer(account.clone(), account.clone().to_string(), None, None);
    }

    //View function
    pub fn cert_lists(&self) -> Vec<(ValidAccountId, Certificate)> {
        return self
            .certs_map
            .iter()
            .collect();
    }

    //Helper function
    fn only_owner(&self) {
        let predecessor = env::predecessor_account_id();
        let receiver_id = ValidAccountId::try_from(predecessor.clone()).unwrap();

        assert_eq!(
            &receiver_id,
            &self.contract_foundation,
            "Only contract owner can call this fn"
            );
    }

    fn only_issuer(&self) {
        let signer = ValidAccountId::try_from(env::predecessor_account_id().clone()).unwrap();

        assert!(
            self.issuers.get(&signer).is_some(),
            "Only called by issuers"
            );
    }
}

near_contract_standards::impl_non_fungible_token_core!(Contract, nft_token);
near_contract_standards::impl_non_fungible_token_approval!(Contract, nft_token);
near_contract_standards::impl_non_fungible_token_enumeration!(Contract, nft_token);

#[near_bindgen]
impl NonFungibleTokenMetadataProvider for Contract {
    fn nft_metadata(&self) -> NFTContractMetadata {
        self.metadata.get().unwrap()
    }
}
