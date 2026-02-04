use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Currency {
    pub name: String,           // Currency name (e.g., "Gold", "Gems")
    pub symbol: String,         // Symbol (e.g., "G", "💎")
    pub exchange_rate: f32,     // Exchange rate for conversions (1 unit in base currency)
    pub max_limit: Option<i64>, // Maximum balance, if applicable
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Wallet {
    pub balances: FxHashMap<String, i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Currencies {
    pub currencies: FxHashMap<String, Currency>, // Map of symbol to Currency
    pub base_currency: String,                   // Symbol of the base currency
}

impl Currencies {
    /// Add a new currency
    pub fn add_currency(&mut self, currency: Currency) -> Result<(), String> {
        if self.currencies.contains_key(&currency.symbol) {
            return Err(format!("Currency {} already exists.", currency.symbol));
        }
        self.currencies.insert(currency.symbol.clone(), currency);
        Ok(())
    }

    /// Get a reference to a currency by symbol
    pub fn get_currency(&self, symbol: &str) -> Option<&Currency> {
        self.currencies.get(symbol)
    }

    /// Convert an amount in base currency to a specific currency
    pub fn convert_from_base(&self, base_amount: i64, to_symbol: &str) -> Result<i64, String> {
        let to_currency = self
            .get_currency(to_symbol)
            .ok_or(format!("Currency {} not found.", to_symbol))?;
        let converted_amount =
            (base_amount as f64 / to_currency.exchange_rate as f64).round() as i64;
        Ok(converted_amount)
    }

    /// Convert an amount in a specific currency to the base currency
    pub fn convert_to_base(&self, amount: i64, from_symbol: &str) -> Result<i64, String> {
        let from_currency = self
            .get_currency(from_symbol)
            .ok_or(format!("Currency {} not found.", from_symbol))?;
        let base_amount = (amount as f64 * from_currency.exchange_rate as f64).round() as i64;
        Ok(base_amount)
    }
}

impl Wallet {
    /// Add currency in base currency units
    pub fn add_base_currency(
        &mut self,
        base_amount: i64,
        currencies: &Currencies,
    ) -> Result<(), String> {
        if base_amount < 0 {
            return Err("Cannot add a negative amount.".to_string());
        }

        // Get the base currency symbol
        let base_symbol = &currencies.base_currency;

        // Convert base_amount to the base currency symbol, then use `add()` which handles overflow
        self.add(base_symbol, base_amount, currencies)
    }

    /// Add currency to the wallet using the currency symbol, handling overflow into higher currencies
    pub fn add(
        &mut self,
        symbol: &str,
        amount: i64,
        currencies: &Currencies,
    ) -> Result<(), String> {
        if amount < 0 {
            return Err("Cannot add a negative amount.".to_string());
        }

        let mut remaining = amount;
        let mut current_symbol = symbol.to_string();

        while remaining > 0 {
            if let Some(currency) = currencies.get_currency(&current_symbol) {
                let current_balance = self.balances.entry(current_symbol.clone()).or_insert(0);
                let max_addable = currency.max_limit.unwrap_or(i64::MAX) - *current_balance;

                if max_addable >= remaining {
                    *current_balance += remaining;
                    remaining = 0;
                } else {
                    *current_balance += max_addable;
                    remaining -= max_addable;

                    // Convert remaining amount to the next higher currency if possible
                    let base_amount = currencies.convert_to_base(remaining, &current_symbol)?;

                    let next_currency = currencies
                        .currencies
                        .iter()
                        .filter(|(_, c)| c.exchange_rate < currency.exchange_rate)
                        .min_by(|(_, c1), (_, c2)| {
                            c1.exchange_rate.partial_cmp(&c2.exchange_rate).unwrap()
                        });

                    if let Some((next_symbol, _)) = next_currency {
                        current_symbol = next_symbol.clone();
                    } else {
                        return Err("No higher currency available for overflow.".to_string());
                    }

                    remaining = base_amount;
                }
            } else {
                return Err(format!("Currency {} does not exist.", current_symbol));
            }
        }

        Ok(())
    }

    /// Spend funds in the base amount.
    pub fn spend(&mut self, base_amount: i64, currencies: &Currencies) -> Result<(), String> {
        let mut remaining_base = base_amount;

        // Sort currencies by descending exchange_rate (high to low value)
        let mut sorted = currencies.currencies.values().collect::<Vec<_>>();
        sorted.sort_by(|a, b| b.exchange_rate.partial_cmp(&a.exchange_rate).unwrap());

        for currency in sorted {
            let symbol = &currency.symbol;
            if let Some(balance) = self.balances.get_mut(symbol) {
                let available_base = currencies.convert_to_base(*balance, symbol)?;
                let to_spend_base = remaining_base.min(available_base);
                let to_spend = currencies.convert_from_base(to_spend_base, symbol)?;

                *balance -= to_spend;
                remaining_base -= to_spend_base;

                if remaining_base <= 0 {
                    break;
                }
            }
        }

        if remaining_base > 0 {
            return Err("Insufficient funds.".to_string());
        }

        Ok(())
    }

    /// Get the total balance in base currency
    pub fn get_balance(&self, currencies: &Currencies) -> i64 {
        self.balances.iter().fold(0, |acc, (symbol, &amount)| {
            if let Some(currency) = currencies.get_currency(symbol) {
                acc + (amount as f64 * currency.exchange_rate as f64).round() as i64
            } else {
                acc
            }
        })
    }

    /// Check if the wallet can afford an amount in base currency
    pub fn can_afford(&self, base_amount: i64, currencies: &Currencies) -> bool {
        self.get_balance(currencies) >= base_amount
    }
}
