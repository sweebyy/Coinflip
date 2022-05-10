use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{ AccountId, env, log, near_bindgen, Promise, PanicOnDefault};
use near_sdk::json_types::U128;
use near_sdk::json_types::U64;
use near_sdk::collections::*;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct CoinFlip {
    pub history: Vector<String>,
    pub times: Vector<U64>,
    pub user_balance: UnorderedMap<AccountId, u128>,
    pub usersum: u128,
}

#[near_bindgen]
impl CoinFlip {
    #[init]
    pub fn new() -> Self {
        let this = Self {
            history: Vector::new(b"history".to_vec()),
            times: Vector::new(b"times".to_vec()),
            user_balance: UnorderedMap::new(b"user_balance".to_vec()),
            usersum: 0,
        };
        this
    }

    pub fn get_user_balance(&self, user_id: AccountId) -> U128 {
        match self.user_balance.get(&user_id) {
            Some(_balance) => {
                return U128::from(_balance);
            }
            None => {
                return U128::from(0);
            }
        }
    }

    pub fn get_total_balance(&self) -> U128 {
        U128::from(env::account_balance())
    }

    pub fn get_history(&self) -> Vec<String> {
        self.history.to_vec()
    }

    pub fn get_times(&self) -> Vec<U64> {
        self.times.to_vec()
    }

    pub fn get_current_timestamp(&self) -> U64 {
        U64::from(env::block_timestamp())
    }

    pub fn get_bettable(&self) -> U128 {
        U128::from(env::account_balance() - self.usersum)
    }

    pub fn get_usersum(&self) -> U128 {
        U128::from(self.usersum)
    }

    pub fn flip(&mut self, head: bool, amount: U128) -> bool {
        // lets flip
        let mut coin_flip_res = env::block_timestamp() % 97 < 48;
        
        // make result tags
        let is_matched = coin_flip_res == head;
        let won_or_lost = if is_matched {
            "won".to_string()
        } else {
            "lost".to_string()
        };
        let head_or_tail = if head {
            "HEADS".to_string()
        } else {
            "TAILS".to_string()
        };
        let amount_divided = amount.0 / 1_000_000_000_000_000_000_000;
        let amount_i32: i32 = amount_divided as i32;
        let amount_fixed_3 = format!("{:.3}", f64::from(amount_i32) / f64::from(1000));

        let log_message = format!(
            "{} flipped {} betting {} Ⓝ  and {}",
            env::predecessor_account_id(),
            head_or_tail,
            amount_fixed_3,
            won_or_lost,
        );

        // store and print the result
        log!(log_message);
        self.times.push(&U64::from(env::block_timestamp()));
        self.history.push(&log_message);

        // distribute taxes
        let tax1 = amount.0 * 25  / 10000;
        let tax2 = amount.0 * 249 / 10000;
        if tax1 > 0 {
            Promise::new("team-pdm.near".parse().unwrap()).transfer(tax1);
            Promise::new("coinflip-pdm.near".parse().unwrap()).transfer(tax1);
        }
        if tax2 > 0 {
            Promise::new("community-fees-pdm.near".parse().unwrap()).transfer(tax2);
        }
        let post_amount = amount.0 - tax1 * 2 - tax2;

        self.usersum -= amount.0;
        // Update user_balance
        let caller = env::predecessor_account_id();
        match self.user_balance.get(&caller) {
            Some(_balance) => {
                self.user_balance.insert(&caller, &(_balance-amount.0));
            }
            None => {
                ()
            }
        }
        // payback funds.
        if is_matched {
            self.usersum += post_amount * 2;
            match self.user_balance.get(&caller) {
                Some(_balance) => {
                    self.user_balance.insert(&caller, &(_balance+post_amount * 2));
                }
                None => {
                    ()
                }
            }
        } else {
            log!("ERR_PUZZLE_SOLVED");
        }
        is_matched
    }

    #[payable]
    pub fn user_deposit(&mut self) {
        let caller = env::predecessor_account_id();
        let amount = env::attached_deposit();
        self.usersum += amount;
        match self.user_balance.get(&caller) {
            Some(_balance) => {
                self.user_balance.insert(&caller, &(_balance+amount));
            },
            None => {
                self.user_balance.insert(&caller, &(amount));
            }
        }
        log!(format!("Attatched funds by user: {:#?}", amount));
    }

    
    pub fn user_withdraw(&mut self, to: AccountId, amount: U128) {
        Promise::new(to.into()).transfer(amount.0);

        let caller = env::predecessor_account_id();
        // let amount = env::attached_deposit();
        self.usersum -= amount.0;
        match self.user_balance.get(&caller) {
            Some(_balance) => {
                self.user_balance.insert(&caller, &(_balance-amount.0));
            },
            None => {
                ()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, VMContext};

    // Deposit money to coinflip contract.
    
    
    // part of writing unit tests is setting up a mock context
    // in this example, this is only needed for log! in the contract
    // this is also a useful list to peek at when wondering what's available in env::*
    fn get_context(input: Vec<u8>, is_view: bool) -> VMContext {

        let ACCESS_KEY_ALLOWANCE: u128 = 1_000_000_000_000_000_000_000_000;
        let deposit = ACCESS_KEY_ALLOWANCE * 100;
        VMContext {
            current_account_id: "alice.testnet".parse().unwrap(),
            signer_account_id: "robert.testnet".parse().unwrap(),
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id: "jane.testnet".parse().unwrap(),
            input,
            block_index: 0,
            block_timestamp: 0,
            account_balance: deposit,
            account_locked_balance: 0,
            storage_usage: 0,
            attached_deposit: ACCESS_KEY_ALLOWANCE,
            prepaid_gas: 10u64.pow(18),
            random_seed: vec![0, 1, 2],
            output_data_receivers: vec![],
            epoch_height: 19,
            view_config: None,
        }
    }

    #[test]
    fn user_deposit_withdraw_test() {
        let context = get_context(vec![], false);
        testing_env!(context);
        let mut contract = CoinFlip::new("jane.testnet".parse().unwrap());
        // Now, call flip func of link drop contract.
        println!("User balance before deposit: {:#?}", contract.get_user_balance());
        println!("total balance before deposit: {:#?}", contract.get_total_balance());
        contract.user_deposit();
        println!("User balance after 1st deposit: {:#?}", contract.get_user_balance());
        println!("total balance after 1st deposit: {:#?}", contract.get_total_balance());
        contract.user_deposit();
        println!("User balance after 2nd deposit: {:#?}", contract.get_user_balance());
        println!("total balance after 2nd deposit: {:#?}", contract.get_total_balance());
        contract.user_withdraw("bob.testnet".parse().unwrap(), 2_000_000_000_000_000_000_000_000);
        println!("User balance after withdraw: {:#?}", contract.get_user_balance());
        println!("total balance after withdraw: {:#?}", contract.get_total_balance());
    }

    #[test]
    fn owner_deposit_withdraw_test() {
        let context = get_context(vec![], false);
        testing_env!(context);
        let mut contract = CoinFlip::new("jane.testnet".parse().unwrap());
        // Now, call flip func of link drop contract.
        println!("total balance before deposit: {:#?}", contract.get_total_balance());
        contract.deposit();
        println!("total balance after 1st deposit: {:#?}", contract.get_total_balance());
        contract.deposit();
        println!("total balance after 2nd deposit: {:#?}", contract.get_total_balance());
        contract.withdraw("bob.testnet".parse().unwrap(), 2_000_000_000_000_000_000_000_000);
        println!("total balance after withdraw: {:#?}", contract.get_total_balance());
    }

    #[test]
    fn not_owner_deposit_withdraw_test() {
        let context = get_context(vec![], false);
        testing_env!(context);
        let mut contract = CoinFlip{
            history: Vector::new(b"history".to_vec()),
            times: Vector::new(b"times".to_vec()),
            user_balance: UnorderedMap::new(b"user_balance".to_vec()),
            owner_id: "bob.testnet".parse().unwrap(),
        };
        // Now, call flip func of link drop contract.
        contract.deposit();
        contract.deposit();
        contract.withdraw("bob.testnet".parse().unwrap(), 2_000_000_000_000_000_000_000_000);
        println!("total balance after withdraw: {:#?}", contract.get_total_balance());
    }

    #[test]
    fn flip_test() {
        let context = get_context(vec![], false);
        testing_env!(context);
        let mut contract = CoinFlip::new("jane.testnet".parse().unwrap());
        // Now, call flip func of link drop contract.
        println!("User balance before deposit: {:#?}", contract.get_user_balance());
        contract.user_deposit();
        println!("User balance after deposit: {:#?}", contract.get_user_balance());
        contract.flip(true, 1_000_000_000_000_000_000_000_000);
        println!("User balance after flip: {:#?}", contract.get_user_balance());
        println!("History after flip: {:#?}", contract.get_history());
        println!("Times after flip: {:#?}", contract.get_times());
        println!("Near Balance after flip: {:#?}", contract.get_total_balance());
    }
}