use ethabi::{decode, ParamType, Token};
use hex;
use keccak_hash;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct JsonRpcRequest {
    id: i32,
    jsonrpc: String,
    method: String,
    params: Vec<serde_json::Value>,
}

fn decode_uint(hex_str: &str) -> u64 {
    let hex_str = hex_str.trim_start_matches("0x");
    u64::from_str_radix(hex_str, 16).unwrap_or(0)
}

fn decode_address(hex_str: &str) -> String {
    let hex_str = hex_str.trim_start_matches("0x");
    format!("0x{}", &hex_str[24..64])
}

fn decode_string(hex_str: &str) -> String {
    if hex_str.len() < 130 {
        return String::from("Invalid data");
    }
    let hex_str = hex_str.trim_start_matches("0x");
    let offset = usize::from_str_radix(&hex_str[0..64], 16).unwrap_or(0);
    let length_hex = &hex_str[offset*2..offset*2+64];
    let length = usize::from_str_radix(length_hex, 16).unwrap_or(0);
    let string_data = &hex_str[offset*2+64..offset*2+64+length*2];
    String::from_utf8(
        hex::decode(string_data).unwrap_or_default()
    ).unwrap_or_else(|_| String::from("Invalid UTF-8"))
}

async fn query(client: &reqwest::Client, rpc_url: &str, contract_address: &str, data: &Vec<u8>) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let data = String::from_utf8(data.to_vec()).unwrap();
    
    let request_body = JsonRpcRequest {
        id: 1,
        jsonrpc: "2.0".to_string(),
        method: "eth_call".to_string(),
        params: vec![
            serde_json::json!({
                "to": contract_address,
                "data": data,
            }),
            serde_json::json!("latest"),
        ],
    };
    let response = client
        .post(rpc_url)
        .json(&request_body)
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .send()
        .await?;
    let result: serde_json::Value = response.json().await?;
    Ok(result)
}

fn encode_function_call(method_signature: &str, params: Vec<String>) -> String {
    let selector = keccak_hash::keccak(method_signature.as_bytes());
    let selector = hex::encode(&selector[0..4]);
    
    let mut encoded_params = String::new();
    
    if method_signature.contains("string") {
        if method_signature.contains(",uint256,uint256") {
            let subject = &params[0];
            let offset = &params[1];
            let limit = &params[2];
            
            encoded_params.push_str(&format!("{:0>64}", "60"));
            encoded_params.push_str(&format!("{:0>64}", offset));
            encoded_params.push_str(&format!("{:0>64}", limit));
            encoded_params.push_str(&format!("{:0>64}", format!("{:x}", subject.len())));
            let mut hex_string = hex::encode(subject.as_bytes());
            while hex_string.len() % 64 != 0 {
                hex_string.push('0');
            }
            encoded_params.push_str(&hex_string);
        } else {
            let param = &params[0];
            encoded_params.push_str(&format!("{:0>64}", "20"));
            encoded_params.push_str(&format!("{:0>64}", format!("{:x}", param.len())));
            let mut hex_string = hex::encode(param.as_bytes());
            while hex_string.len() % 64 != 0 {
                hex_string.push('0');
            }
            encoded_params.push_str(&hex_string);
        }
    } else {
        for param in params {
            if param.starts_with("0x") {
                encoded_params.push_str(&param[2..]);
            } else {
                encoded_params.push_str(&format!("{:0>64}", param));
            }
        }
    }
    
    format!("0x{}{}", selector, encoded_params)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    const RPC_URL: &str = "https://sepolia.drpc.org";
    const CONTRACT_ADDRESS: &str = "0x1c7D4B196Cb0C7B01d743Fbc6116a902379C7238";

    println!("\n-------FT ERC20 CONTRACT-------\n");
    // Get token name
    let name_data = encode_function_call("name()", vec![]).into_bytes();

    let name_response = query(&client, RPC_URL, CONTRACT_ADDRESS, &name_data).await?;
    let name = decode_string(name_response["result"].as_str().unwrap_or_default());
    println!("Name: {}", name);

    // Get token symbol
    let symbol_data = encode_function_call("symbol()", vec![]).into_bytes();
    let symbol_response = query(&client, RPC_URL, CONTRACT_ADDRESS, &symbol_data).await?;
    let symbol = decode_string(symbol_response["result"].as_str().unwrap_or_default());
    println!("Symbol: {}", symbol);

    // Get decimals
    let decimals_data = encode_function_call("decimals()", vec![]).into_bytes();
    let decimals_response = query(&client, RPC_URL, CONTRACT_ADDRESS, &decimals_data).await?;
    let decimals = decode_uint(decimals_response["result"].as_str().unwrap_or_default());
    println!("Decimals: {}", decimals);

    // Get total supply
    let total_supply_data = encode_function_call("totalSupply()", vec![]).into_bytes();
    let total_supply_response = query(&client, RPC_URL, CONTRACT_ADDRESS, &total_supply_data).await?;
    let total_supply = decode_uint(total_supply_response["result"].as_str().unwrap_or_default());
    println!("Total Supply: {}", total_supply);

    // Get balance of the contract
    let balance_data = encode_function_call(
        "balanceOf(address)",
        vec![format!("{:0>64}", CONTRACT_ADDRESS.trim_start_matches("0x"))]
    ).into_bytes();
    let balance_response = query(&client, RPC_URL, CONTRACT_ADDRESS, &balance_data).await?;
    let balance = decode_uint(balance_response["result"].as_str().unwrap_or_default());
    println!("Balance: {}", balance);

    println!("\n-------NFT CONTRACT-------\n");
    const NFT_ADDRESS: &str = "0x1238536071E1c677A632429e3655c799b22cDA52";

    // Get NFT name
    let nft_name_data = encode_function_call("name()", vec![]).into_bytes();
    let nft_name_response = query(&client, RPC_URL, NFT_ADDRESS, &nft_name_data).await?;
    let nft_name = decode_string(nft_name_response["result"].as_str().unwrap_or_default());
    println!("NFT Name: {}", nft_name);

    // Get NFT symbol
    let nft_symbol_data = encode_function_call("symbol()", vec![]).into_bytes();
    let nft_symbol_response = query(&client, RPC_URL, NFT_ADDRESS, &nft_symbol_data).await?;
    let nft_symbol = decode_string(nft_symbol_response["result"].as_str().unwrap_or_default());
    println!("NFT Symbol: {}", nft_symbol);

    // Get total supply of NFTs
    let nft_supply_data = encode_function_call("totalSupply()", vec![]).into_bytes();
    let nft_supply_response = query(&client, RPC_URL, NFT_ADDRESS, &nft_supply_data).await?;
    let nft_supply = decode_uint(nft_supply_response["result"].as_str().unwrap_or_default());
    println!("Total NFTs: {}", nft_supply);

    // Get owner of token ID 1
    let token_id = format!("{:0>64}", "1"); // Pad token ID 1 to 64 characters
    let owner_data = encode_function_call("ownerOf(uint256)", vec![token_id.clone()]).into_bytes();
    let owner_response = query(&client, RPC_URL, NFT_ADDRESS, &owner_data).await?;
    let owner = decode_address(owner_response["result"].as_str().unwrap_or_default());
    println!("Owner of Token #1: {}", owner);

    // Get balance of NFTs for the contract address
    let nft_balance_data = encode_function_call(
        "balanceOf(address)",
        vec![format!("{:0>64}", NFT_ADDRESS.trim_start_matches("0x"))]
    ).into_bytes();
    let nft_balance_response = query(&client, RPC_URL, NFT_ADDRESS, &nft_balance_data).await?;
    let nft_balance = decode_uint(nft_balance_response["result"].as_str().unwrap_or_default());
    println!("NFT Balance: {}", nft_balance);

    // Get token URI
    let token_uri_data = encode_function_call("tokenURI(uint256)", vec![token_id]).into_bytes();
    let token_uri_response = query(&client, RPC_URL, NFT_ADDRESS, &token_uri_data).await?;
    let _token_uri = decode_string(token_uri_response["result"].as_str().unwrap_or_default());
    //println!("Token #1 URI: {}", token_uri);

    // Subjects with students Sepolia -> Map<String, String[]>
    println!("\n-------SUBJECT CONTRACT-------\n");
    const SUBJECT_CONTRACT: &str = "0x5a9491e24f9de0dc6a82e280da939bf36269c48e";

    let subject = "Mathematics";
    let function_signature = "getStudentCount(string)";

    let count_data = encode_function_call(
        function_signature,
        vec![subject.to_string()],
    ).into_bytes();
    let count_response = query(&client, RPC_URL, SUBJECT_CONTRACT, &count_data).await?;
    let student_count = decode_uint(count_response["result"].as_str().unwrap_or_default());
    println!("Number of students in {}: {}", subject, student_count);

    // Get students by subject
    let offset = 0;
    let limit = 10;
    let get_students_data = encode_function_call(
        "getStudentsBySubject(string,uint256,uint256)",
        vec![
            subject.to_string(),
            offset.to_string(),
            limit.to_string(),
        ]
    ).into_bytes();
    let students_response = query(&client, RPC_URL, SUBJECT_CONTRACT, &get_students_data).await?;

    println!("Students response: {:?}", decode_students_response(students_response["result"].as_str().unwrap_or_default()));

    Ok(())
}


fn decode_students_response(response: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let response_data = if response.starts_with("0x") {
        &response[2..]
    } else {
        response
    };

    let bytes = hex::decode(response_data)?;
    let decoded = decode(
        &[ParamType::Array(Box::new(ParamType::String))],
        &bytes,
    )?;

    if let Token::Array(tokens) = &decoded[0] {
        let students = tokens
            .iter()
            .filter_map(|token| {
                if let Token::String(value) = token {
                    Some(value.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        Ok(students)
    } else {
        Err("Unexpected response format".into())
    }
}