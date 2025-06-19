use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, interval};
use pyth_hermes_client::PythClient;
use futures::stream::StreamExt; // For stream processing methods
use blackscholes::{Inputs, OptionType, Pricing};


// Core data structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceData {
    pub symbol: String,
    pub price: f64,
    pub timestamp: i64,
    pub confidence: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionParams {
    pub underlying_price: f64,
    pub strike_price: f64,
    pub time_to_expiry: i64, // in years
    pub volatility: f64,
    pub risk_free_rate: f64,
    pub is_call: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PremiumResult {
    pub strike: f64,
    pub premium: f64,
    pub timestamp: u64,
}

// Black-Scholes implementation
pub struct BlackScholes;

impl BlackScholes {
    pub fn calculate_premium(params: &OptionParams) -> f64 {
        // Convert time_to_expiry from days to years (assuming input is in days)
        let time_in_years = params.time_to_expiry as f32 / 365.25;
        
        // Determine option type
        let option_type = if params.is_call {
            OptionType::Call
        } else {
            OptionType::Put
        };
        
        // Create inputs for black-scholes calculation
        let inputs = Inputs::new(
            option_type,                     // Call or Put
            params.underlying_price as f32,  // Current price (S)
            params.strike_price as f32,      // Strike price (K)
            None,                            // Premium (not used for pricing)
            params.risk_free_rate as f32,    // Risk-free rate
            0.0,                             // Dividend yield (typically 0 for crypto)
            time_in_years,                   // Time to maturity in years
            Some(params.volatility as f32),  // Volatility
        );
        
        // Calculate and return the option price
        let price: f32 = inputs.calc_price().unwrap();
        price as f64 // Convert back to f64 for consistency
    }
}

// Pyth Oracle Provider for Sui
pub struct PythOracle {
    // In a real implementation, you'd have Sui client and Pyth price feed IDs
    price_feeds: HashMap<String, String>,
    client: pyth_hermes_client::PythClient, // symbol -> feed_id
   
}

impl PythOracle {
    pub fn new() -> Self {
        let mut price_feeds = HashMap::new();
        
        // // Example Pyth price feed IDs (these are examples, use actual ones)
        // price_feeds.insert("BTC".to_string(), "0xe62df6c8b4c85fe1c755c63f0e2e6a1e8b8d8a2d".to_string());
        // price_feeds.insert("ETH".to_string(), "0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace".to_string());
        price_feeds.insert("SUI".to_string(), "0x23d7315113f5b1d3ba7a83604c44b94d79f4fd69af77f804fc7f920a6dc65744".to_string());
        let client = PythClient::new(
            "https://hermes.pyth.network".parse().unwrap()
        );
        Self {
            price_feeds,
            client,
        }
    }

    

    pub async fn fetch_volatility(&self, symbol: &str) -> Result<f64> {
        // Mock volatility calculation - in practice you'd calculate from historical data
        let base_vol = match symbol {
            "BTC" => 0.8,
            "ETH" => 0.9,
            "SUI" => 1.2,
            _ => 1.0,
        };

        Ok(base_vol)
    }


    pub async fn fetch_pyth_price_real(&self, feed_id: &str) -> Result<PriceData> {
        let mut price_updates = self.client.stream_price_updates(
            vec![feed_id.to_string()], 
            None, 
            Some(true), 
            Some(false), 
            Some(true)
        ).await?;
        
        // Process the stream until we get the first valid price
        loop {
            match price_updates.next().await {
                Some(Ok(price_update)) => {
                    println!("Received price update:");
                    
                    // Access parsed data if available
                    if let Some(parsed) = &price_update.parsed {
                        for price_feed in parsed {
                            let price = price_feed.price.price as f64 * 10.0_f64.powi(price_feed.price.expo);

                            let symbol = self.price_feeds.iter()
                            .find_map(|(key, val)| if *val ==feed_id { Some(key.clone()) } else { None })
                            .unwrap_or_else(|| "UNKNOWN".to_string()); 
                            // Return the first valid price data we receive
                            return Ok(PriceData {
                                symbol: symbol,
                                price,
                                timestamp: price_feed.price.publish_time as i64,
                                confidence: price_feed.price.conf as i64,
                            });
                        }
                    }
                },
                Some(Err(err)) => {
                    eprintln!("Error receiving price update: {:?}", err);
                    // Continue to next update instead of failing immediately
                    continue;
                },
                None => {
                    // Stream ended
                    break;
                }
            }
        }
        
        // If we exit the loop without finding data
        Err(anyhow::anyhow!("Failed to fetch price data - stream ended without valid data"))
    }   



}
// Simple Options Pricing Engine
pub struct OptionsPricingEngine {
    oracle: Arc<PythOracle>,
    config: EngineConfig,
    last_prices: Arc<RwLock<HashMap<String, PriceData>>>,
}

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub risk_free_rate: f64,
    pub default_volatility: f64,
    pub update_interval_secs: u64,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            risk_free_rate: 0.05, // 5%
            default_volatility: 0.8, // 80%
            update_interval_secs: 10,
        }
    }
}

impl OptionsPricingEngine {
    pub fn new(oracle: Arc<PythOracle>, config: EngineConfig) -> Self {
        Self {
            oracle,
            config,
            last_prices: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start_price_updates(&self, symbols: Vec<String>) {
        let oracle = self.oracle.clone();
        let last_prices = self.last_prices.clone();
       let last_price=self.oracle.fetch_pyth_price_real(&self.oracle.price_feeds[&symbols[0]]).await.unwrap();
       //write to last_prices for latest timestamp and pop the the most old element in it 
        {
            let mut prices = last_prices.write().unwrap();
            if prices.len() >= 10 { // Limit to 10 most recent prices
                if let Some(key_to_remove) = prices.keys().next().cloned() {
                    prices.remove(&key_to_remove); // Remove the oldest
                }
            }
        }
        last_prices.write().unwrap().insert(symbols[0].clone(), last_price);
  
    }

    pub async fn calculate_option_premium(
        &self,
        symbol: &str,
        strike: f64,
        days_to_expiry: u32,
        is_call: bool,
    ) -> Result<PremiumResult> {
        // // Get current price
        // let price_data = {
        //     let prices = self.last_prices.read().unwrap();
        //     prices.get(symbol).cloned()
        // };
        //get current price from oracle
        let price_data = self.oracle.fetch_pyth_price_real(&self.oracle.price_feeds[symbol]).await
            .context(format!("Failed to fetch price for symbol: {}", symbol))?;

        // let underlying_price = match price_data {
        //     Some(data) => data.price,
        //     None => {
        //         // Fetch fresh price if not cached
        //         let fresh_data = self.oracle.fetch_pyth_price_real(symbol).await?;
        //         let price = fresh_data.price;
        //         self.last_prices.write().unwrap().insert(symbol.to_string(), fresh_data);
        //         price
        //     }
        // };

        // Get volatility
        let volatility = self.oracle.fetch_volatility(symbol).await
            .unwrap_or(self.config.default_volatility);
         
         let underlying_price = price_data.price;
        // Calculate premium
        let params = OptionParams {
            underlying_price,
            strike_price: strike,
            time_to_expiry: days_to_expiry as i64 ,
            volatility,
            risk_free_rate: self.config.risk_free_rate,
            is_call,
        };

        let premium = BlackScholes::calculate_premium(&params);

        Ok(PremiumResult {
            strike,
            premium,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        })
    }

    pub async fn calculate_premium_curve(
        &self,
        symbol: &str,
        days_to_expiry: u32,
        strike_range: (f64, f64, f64), // (min, max, step)
    ) -> Result<Vec<PremiumResult>> {
        let mut results = Vec::new();
        let (min_strike, max_strike, step) = strike_range;

        let mut current_strike = min_strike;
        while current_strike <= max_strike {
            // Calculate both call and put premiums
            let call_premium = self
                .calculate_option_premium(symbol, current_strike, days_to_expiry, true)
                .await?;
                
            let put_premium = self
                .calculate_option_premium(symbol, current_strike, days_to_expiry, false)
                .await?;

            results.push(PremiumResult {
                strike: current_strike,
                premium: call_premium.premium,
                timestamp: call_premium.timestamp,
            });

            results.push(PremiumResult {
                strike: -current_strike, // Negative to indicate put
                premium: put_premium.premium,
                timestamp: put_premium.timestamp,
            });

            current_strike += step;
        }

        Ok(results)
    }

    pub fn get_last_price(&self, symbol: &str) -> Option<PriceData> {
        self.last_prices.read().unwrap().get(symbol).cloned()
    }
}

// Example usage and testing
#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting Options Pricing Engine");

    // Initialize oracle and engine
    let oracle = Arc::new(PythOracle::new());
    let config = EngineConfig::default();
    let engine = OptionsPricingEngine::new(oracle, config);

    // Start price updates for key symbols
    let symbols = vec!["BTC".to_string(), "ETH".to_string(), "SUI".to_string()];
    engine.start_price_updates(symbols).await;

    // Wait a bit for prices to be fetched
    sleep(Duration::from_secs(2)).await;

    // Example: Calculate option premium for BTC
    println!("\n=== BTC Option Pricing ===");
    let btc_call = engine
        .calculate_option_premium("BTC", 50000.0, 7, true)
        .await?;
    
    println!("BTC $50,000 Call (7 days): ${:.2}", btc_call.premium);

    let btc_put = engine
        .calculate_option_premium("BTC", 40000.0, 7, false)
        .await?;
    
    println!("BTC $40,000 Put (7 days): ${:.2}", btc_put.premium);

    // Example: Generate premium curve
    println!("\n=== ETH Premium Curve ===");
    let eth_price = engine.get_last_price("ETH").unwrap().price;
    let curve_range = (eth_price * 0.8, eth_price * 1.2, eth_price * 0.05);
    
    let curve = engine
        .calculate_premium_curve("ETH", 30, curve_range)
        .await?;

    println!("Generated {} premium points", curve.len());
    for point in curve.iter().take(5) {
        let option_type = if point.strike > 0.0 { "Call" } else { "Put" };
        let strike = point.strike.abs();
        println!("{} ${:.0}: ${:.2}", option_type, strike, point.premium);
    }

    // Keep running
    println!("\nEngine running... Press Ctrl+C to stop");
    loop {
        sleep(Duration::from_secs(10)).await;
        
        // Show current prices
        if let Some(btc_price) = engine.get_last_price("BTC") {
            println!("BTC: ${:.2}", btc_price.price);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_black_scholes_call() {
        let params = OptionParams {
            underlying_price: 100.0,
            strike_price: 100.0,
            time_to_expiry: 1111,
            volatility: 0.2,
            risk_free_rate: 0.05,
            is_call: true,
        };

        let premium = BlackScholes::calculate_premium(&params);
        assert!(premium > 0.0);
        assert!(premium < 50.0); // ATM call should be reasonable
    }

    #[test]
    fn test_black_scholes_put() {
        let params = OptionParams {
            underlying_price: 100.0,
            strike_price: 100.0,
            time_to_expiry: 1111,
            volatility: 0.2,
            risk_free_rate: 0.05,
            is_call: false,
        };

        let premium = BlackScholes::calculate_premium(&params);
        println!("Put Premium: {}", premium);
        println!("call premium: {}", BlackScholes::calculate_premium(&OptionParams {
            underlying_price: 100.0,
            strike_price: 100.0,
            time_to_expiry: 1111,
            volatility: 0.2,
            risk_free_rate: 0.05,
            is_call: true,
        }));
        assert!(premium > 0.0);
        assert!(premium < 50.0);
    }

    #[tokio::test]
    async fn test_oracle_price_fetch() {
        let oracle = PythOracle::new();
        println!("{}",oracle.price_feeds["SUI"].as_str());
        let price = oracle.fetch_pyth_price_real(oracle.price_feeds["SUI"].as_str()).await.unwrap();
        println!("Fetched Price: {:?}", price);
        assert_eq!(price.symbol, "SUI");
        assert!(price.price > 0.0);
        assert!(price.confidence > 0);
    }

    #[tokio::test]
    async fn test_engine_premium_calculation() {
        let oracle = Arc::new(PythOracle::new());
        let engine = OptionsPricingEngine::new(oracle, EngineConfig::default());
        
        let result = engine
            .calculate_option_premium("SUI", 2.89, 1, true)
            .await
            .unwrap();
            
        println!("Calculated Premium: ${:.2}", result.premium);
        assert!(result.premium > 0.0);
        assert!(result.strike > 0.0);
    }
}