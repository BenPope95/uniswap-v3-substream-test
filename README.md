In this project I was attempting to modify the Uniswap V3 Substream to grab token info from storage changes in token deployments caused by the constructor function and storing them for later use. 

## Original Substream 
Original Substream by streamingfast located at https://github.com/streamingfast/substreams-uniswap-v3

## Goals of This Substream
To increase long term efficiency in the uniswap V3 substream by reducing the amount of rpc calls used. 

## Modules Added
1. map_token_deployments 
2. store_token_deployments

## Modules Modified
1. map_pools_created

## map_token_deployments
`pub fn map_token_deployments(block: Block) -> Result<Erc20Tokens, Error> {`

The map_token_deployments module is the first module I have added. It takes in an ethereum block as an input and returns a result type with the happy path being `Erc20Tokens`,  a struct that contains a vector of `Erc20Token` structs. 

The first thing I did in this module is define an empty mutable vector name token_deployments. This is where i will add my Erc20Token structs that I find. 

``` rust
let mut token_deployments = vec![];
```

I then begin looping over the logs in the block and filter by the length of the transaction input and the length of the topics in the logs. Checking for the transaction input length is a way to filter contract deployments from normal transactions since the deployment bytecode will be included in the transaction input. I check for the topics to have a length of 3 because i am looking for logs that include a ERC20 transfer event which has 3 topics.

``` rust
for logview in block.logs() {
	if logview.receipt.transaction.input.len() > 1000 && 
	logview.log.topics.len() == 3 {
```

I then check that Topic 0 equals the event signature for a Transfer event.  Next I grab the log transaction hash for later and check that Topic 1 which is the from address equals address zero. This indicates tokens being minted which helps me identify a token deployment. Once I have confirmed these conditions I declare mutable variables for the token name, symbol, and address as well as a boolean flag that will be used to prevent duplicates when grabbing the token info.

``` rust
let topic_0 = format_hex(&logview.log.topics[0]);
if &topic_0 == TRANSFER_EVENT_SIG {
	let log_tx_hash = format_hex(&logview.receipt.transaction.hash);
	let from_address = format_hex(&logview.log.topics[1]);
	if from_address == ADDRESS_ZERO {
		let mut found_token_info: bool = false;
		let mut token_name = String::new();
		let mut token_symbol = String::new();
		let mut token_address = String::new();
```

I now check that the log transaction hash equals the call transaction hash. This ensures that the storage changes i am grabbing from the call are from the same transaction as when the tokens are minted.  

``` rust
for callview in block.calls() {
	let call_tx_hash = format_hex(&callview.transaction.hash);
	if call_tx_hash == log_tx_hash {
		let storage_changes = &callview.call.storage_changes;
```

### Pattern Matching

Here is where things start to get a little more complicated. I am attempting to grab the token name and symbol from storage changes set in the constructor function during token contract deployments.  Token contracts can vary widely in their implementation which means that finding a one-size-fits all solution for extracting token info is very challenging. Because I am grabbing the token info through storage changes, I need a way to consistently grab the storage changes at the correct storage slot for the token name and symbol, even if those are located in different storage slots in different contracts. I realized that creating a perfect solution that grabs the data I want from every single token deployment was not feasible. With this is mind, my goal was to create a solution that would capture a substantial number of tokens in order to make a significant performance increase.

This led me to focus on a specific subset of token contracts. Ones that are based on the OpenZeppelin ERC20 standard. The reason for this is that in the ERC20.sol contract the token name and symbol are set in a predictable order in the constructor, with the name being set first and the symbol being set immediately after. This gives us a consistent pattern to look for when going over the storage changes.  

``` solidity
ERC20.sol
 
 
 constructor(string memory name_, string memory symbol_) {
        _name = name_;
        _symbol = symbol_;
    }
```

The next part of the code involves looping over the storage changes to try to extract the token info. Since the storage slots for the name and symbol aren't set in stone across different contracts, I had to get a bit creative in my approach. 

Because I know that in ERC20.sol always sets the name and symbol together in a certain order, I can read the value of the storage changes and figure when a symbol is set immediately after the name. To do so I need to recognize the content of the string as either a token name or a token symbol. 

Ok now back where we left off. We now know that our calls should all be from token contract deployments so we can start looping over the vector of storage changes to try to find a match for the name and symbol.

I start by looping over the storage changes and setting variables for the current and previous storage change. I then trim the strings to get rid of any unwanted characters. 

``` rust
let storage_changes = &callview.call.storage_changes;
for i in 1..storage_changes.len() {
	let prev_change = &storage_changes[i - 1].new_value;
	let curr_change = &storage_changes[i].new_value;
	if let (Ok(prev_string), Ok(curr_string)) = (
	String::from_utf8(prev_change.clone()),
	String::from_utf8(curr_change.clone()),
	) {
		let prev_string_trimmed = prev_string
		.chars()
		.filter(|c| {
			c.is_alphanumeric()
				|| c.is_whitespace()
				|| *c == '-'
				|| *c == '$'
				|| *c == '_'
		})
		.collect::<String>();
	let curr_string_trimmed = curr_string
	.chars()
	.filter(|c| c.is_alphanumeric() || *c == '-' || *c == '$' || *c == '_')
	.collect::<String>();```

```

Now that I have my trimmed strings I am ready to compare them and check if the pattern matches a token name and symbol. This is where regex comes into play. In the code block you see below , I'm using Rust's Regex library to work with regular expressions. I define two regex patterns, `token_name_regex` and `token_symbol_regex`, where `token_name_regex` checks if the string consists of alphabetic characters and whitespace only, typically used to validate token names, while `token_symbol_regex` verifies if the string consists of uppercase letters only, commonly used for token symbols. We then apply these regex patterns to the trimmed strings, `prev_string_trimmed` and `curr_string_trimmed`, using `is_match` to determine if the `prev_string_trimmed` matches the expected pattern for a token name, and that `curr_string_trimmed`  matches the expected pattern for a token symbol.

``` rust
if let (Ok(token_name_regex), Ok(token_symbol_regex)) =

(Regex::new(r"^[A-Za-z\s]+$"), Regex::new(r"^[A-Z]+$"))

{

if token_name_regex.is_match(&prev_string_trimmed)

&& token_symbol_regex.is_match(&curr_string_trimmed)

{
```

After checking that the regex patterns match for the name and symbol, I added some more detailed checks to confirm that i have a valid name and symbol and rule out false positives from unrelated strings. I check that the first character of the current string matches the first character of the previous string, that the length of the previous string is greater or equal to the length of the current string, and that the current strings length is greater than 2.  These are all common patterns for token names and symbols that will allow me to grab a substantial amount of tokens while significantly reducing the risk of false positives. If the strings pass all of these checks I set my `found_token_info` boolean to true, set `token_name` to `prev_string_trimmed` , `token_symbol` to `curr_string_trimmed`, and `token_address` to the address of the call the storage changes are from. 

``` rust
{
	if !curr_string_trimmed.is_empty() {
		let first_char =
		curr_string_trimmed.chars().next().unwrap().to_string();
		if prev_string_trimmed.starts_with(&first_char)
	&& prev_string_trimmed.len() >= curr_string_trimmed.len()
	&& curr_string_trimmed.len() > 2
	{
		found_token_info = true;
		token_name = prev_string_trimmed;
		token_symbol = curr_string_trimmed;
		token_address = Hex(&callview.call.address).to_string();

}
```

Once `found_token_info` is set to true we can instantiate our  Erc20Token struct. 
We have to make a rpc call to the token contract to get the decimal value since we cannot apply the same pattern matching logic to the decimals that we used for the name and symbol.  If the rpc call fails to grab the decimal value we skip the token for the sake of keeping our token data consistent.  If the rpc call successful gets the decimal value then we can instantiate our struct with the token data and push it to our `token_deployments` vector. The `total_supply` and `whitelist_pools` remain empty as they will be filled in other map modules downstream. 

``` rust
					}
				}
			}
		}
	}
}

if found_token_info {
	let decimals_value = abi::erc20::functions::Decimals {};
	substreams::log::info!("about to create token");
	if let Some(decimals_result) = match hex::decode(&token_address) {
		Ok(decoded_address) => decimals_value.call(decoded_address),
		Err(_) => {
			substreams::log::info!("failed to decode token address");
			None
		}
	} {
		token_deployments.push(Erc20Token {
			address: token_address.clone(),
			name: token_name.clone(),
			symbol: token_symbol.clone(),
			decimals: decimals_result.to_u64(),
			total_supply: "".to_string(),
			whitelist_pools: vec![],
		});
		substreams::log::info!("grabbed info from storage changes");
		break;
	};
```

Finally we return our `Erc20Tokens` struct which should now contain a vector of valid tokens. 

## store_token_deployments

``` rust
#[substreams::handlers::store]
pub fn store_token_deployments(tokens: Erc20Tokens, store: StoreSetProto<Erc20Token>) {
	for token in tokens.tokens {
		store.set(0, &token.address, &token);
	}
}
```

