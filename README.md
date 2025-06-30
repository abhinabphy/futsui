

# 📈 **futsui — A Hedged DeFi Options Vault Protocol on Sui**

> **Decentralized, risk-managed, dynamic option issuance infrastructure with off-chain Rust-powered pricing and on-chain Move smart contracts. A new primitive for DeFi derivatives on Sui.**

---

## 📌 What is futsui?

**futsui** is a decentralized, smart-contract-based options protocol on the **Sui blockchain**. It allows users to issue, buy, and settle **call and put options** on crypto assets (like SUI tokens), while dynamically pricing those options using a live, off-chain Rust pricing engine and optionally tracking **hedging exposure without requiring protocol-owned liquidity.**

It is designed as a **modular system combining:**

* **On-chain Move contracts** for state management, option issuance, premium handling, and settlement
* **Off-chain Rust services** for real-time option pricing, risk assessment, and price streaming

This separation of concerns enables the protocol to be **capital-efficient**, transparent, and open to future enhancements like social integrations or hedging API partnerships.

---

## 📌 Conceptual Foundations

### 📊 What is an Option?

An **option** is a financial derivative contract giving the buyer the right (but not the obligation) to buy or sell an asset at a predetermined price (called the **strike price**) before or at a set expiry date.

* A **call option**: right to **buy** an asset at strike
* A **put option**: right to **sell** an asset at strike

If the market moves in your favor, you exercise the option and profit. If not, you let it expire worthless.

---

### 📊 What is a Hedged Options Vault?

An **options vault** is a pooled contract where liquidity providers deposit funds into a vault, and the vault issues options against that liquidity.
A **hedged options vault** attempts to offset the risks of these option sales through:

* Actual market hedges (like perps)
* Or **virtual delta exposure accounting** (tracking risk exposure without directly hedging, adjusting pricing or limiting new issuance based on aggregate risk)

**futsui currently implements the second approach**: virtual hedging.

---

## 📈 Why Virtual Hedging?

Active hedging requires capital.
**Virtual hedging** lets you:

* Track your option vault’s delta exposure
* Increase premiums dynamically as risk grows
* Block excessive issuance on heavily exposed strikes
* Maintain financial risk awareness without capital requirements

Future versions can plug this into external perps DEX APIs when live.

---

## 📦 Architecture Overview

```text
Users
 │
 │  issue_call_option()
 │  issue_put_option()
 ▼
OptionsVault.move (Move module)
 │
 │  calls pricing engine if needed
 ▼
PremiumCurve.move (on-chain premium storage)
 │
 ▼
 Settlement.move (settles options at expiry)
 │
 ▼
PriceOracleAdapter.move (fetches TWAP/EMA from off-chain Rust price streamer)
 │
 ▼
Rust Pricing Engine (EMA + Black-Scholes pricing logic)
 │
 ▼
External Price Feed (Pyth / custom APIs)
```

---

## 📌 📦 Move Smart Contract Model (Sui)

Sui’s **Move language** powers the smart contracts.
Each Move module isolates a core business domain:

| Module                    | Responsibility                                       |
| :------------------------ | :--------------------------------------------------- |
| `OptionsVault.move`       | Issues options, collects premiums, handles liquidity |
| `OptionPosition.move`     | NFT-like structs representing issued options         |
| `PremiumCurve.move`       | Stores dynamic premium pricing curves on-chain       |
| `Settlement.move`         | Settles options at expiry, determines payouts        |
| `HedgePosition.move`      | Tracks vault-wide delta exposure virtually           |
| `PriceOracleAdapter.move` | Standardizes price fetching logic for contracts      |

**Benefits:**

* Decentralized, verifiable state and settlement
* Capital efficient: No vault-held hedge liquidity needed
* Modular code, easy to upgrade or extend
* Integrates off-chain risk calculations while keeping state on-chain

---

## 📦 Rust Pricing Engine Model

A high-performance **Rust service** responsible for:

* Streaming live price data via Pyth Hermes
* Maintaining EMA or TWAP pricing snapshots for settlement windows
* Calculating option premiums using **Black-Scholes** or DeFi-native models
* Exposing dynamic premium values via JSON-RPC endpoint or Sui-compatible oracle updater transactions

**Core components:**

| File          | Responsibility                                             |
| :------------ | :--------------------------------------------------------- |
| `pricing.rs`  | Option pricing logic (Black-Scholes + EMA)                 |
| `streamer.rs` | Real-time price streaming and history caching              |
| `main.rs`     | Service bootstrap, JSON-RPC endpoints, Sui RPC interfacing |

**Why Rust?**

* High performance for real-time financial computations
* Async-native for constant streaming and API integrations
* Safe, efficient, minimal runtime overhead

---

## 📊 How Pricing Works

**On option issuance request:**

1. Rust engine fetches EMA/TWAP/spot price
2. Calculates option premium based on:

   * Current volatility estimate (intra-stream)
   * Time to expiry
   * Strike price relative to EMA
   * Vault-wide utilization or risk metrics (via HedgePosition data)
3. Updates premium curve via a Move transaction if necessary
4. Returns premium to the user requestor

**This ensures decentralized issuance at fair market rates** without frontend price manipulation risks.

---

## 📌 Real-World Use Case

**Alice believes SUI price will rise to \$5.00 in 200 days**

* She issues a **call option** via Sui CLI or Twitter agent
* Pricing engine calculates a fair premium
* Option minted and stored in `OptionPosition`
* Option is settled via `Settlement` module at expiry using EMA price
* PnL emitted on-chain → bot posts to Twitter leaderboard

---

## 📢 Optional Social Layer

A future Twitter/Discord agent listens to option events:

* Announces top profits
* Allows option issuance via tweet DMs
* Adds a gamified, social DeFi trading experience
  **Without a frontend.**

---

## 📌 Next Development Milestones

* [x] Finish pricing engine + price streaming
* [x] Implement core Move modules for issuance/settlement
* [ ] Integrate virtual hedging exposure accounting
* [ ] Deploy event listener + Twitter/Discord PnL agent
* [ ] Public testnet vault launch

---

## 📣 Conclusion

**futsui** builds a DeFi-native, capital-efficient, socially-driven options trading primitive for Sui — with dynamic pricing, transparent risk tracking, and future hedging infrastructure, proving you can decentralize advanced derivatives infrastructure without capital lockups or frontend reliance.

---

## 📜 License

MIT

---

## 📣 Contact

[@abhinabphy](https://twitter.com/abhinabphy)
[Open an issue](https://github.com/abhinabphy)

