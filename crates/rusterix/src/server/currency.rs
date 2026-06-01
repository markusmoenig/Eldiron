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
    #[serde(default)]
    pub currency_ids: FxHashMap<String, String>, // Map of ruleset currency id to symbol
}

impl Currencies {
    pub fn official_default() -> Self {
        let mut currencies = Self::default();
        let _ = currencies.add_currency_with_id(
            "copper",
            Currency {
                name: "Copper".into(),
                symbol: "c".into(),
                exchange_rate: 1.0,
                max_limit: None,
            },
        );
        let _ = currencies.add_currency_with_id(
            "silver",
            Currency {
                name: "Silver".into(),
                symbol: "s".into(),
                exchange_rate: 10.0,
                max_limit: None,
            },
        );
        let _ = currencies.add_currency_with_id(
            "gold",
            Currency {
                name: "Gold".into(),
                symbol: "g".into(),
                exchange_rate: 100.0,
                max_limit: None,
            },
        );
        currencies.base_currency = "c".into();
        currencies
    }

    pub fn from_rules_source(source: &str) -> Self {
        source
            .parse::<toml::Table>()
            .ok()
            .as_ref()
            .map(Self::from_rules)
            .unwrap_or_else(Self::official_default)
    }

    pub fn from_rules(rules: &toml::value::Table) -> Self {
        let Some(economy) = rules.get("economy").and_then(toml::Value::as_table) else {
            return Self::official_default();
        };
        let Some(currency_table) = economy.get("currencies").and_then(toml::Value::as_table) else {
            return Self::official_default();
        };

        let mut currencies = Self::default();
        for (id, value) in currency_table {
            let Some(table) = value.as_table() else {
                continue;
            };
            let symbol = table
                .get("symbol")
                .and_then(toml::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(id)
                .to_string();
            let name = table
                .get("name")
                .and_then(toml::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(id)
                .to_string();
            let value = table
                .get("value")
                .and_then(toml::Value::as_float)
                .or_else(|| {
                    table
                        .get("value")
                        .and_then(toml::Value::as_integer)
                        .map(|v| v as f64)
                })
                .unwrap_or(1.0)
                .max(1.0) as f32;
            let max_limit = table
                .get("max")
                .and_then(toml::Value::as_integer)
                .filter(|value| *value >= 0);

            let _ = currencies.add_currency_with_id(
                id,
                Currency {
                    name,
                    symbol,
                    exchange_rate: value,
                    max_limit,
                },
            );
        }

        if currencies.currencies.is_empty() {
            return Self::official_default();
        }

        let base_id = economy
            .get("base")
            .and_then(toml::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("copper");
        currencies.base_currency = currencies
            .symbol_for(base_id)
            .or_else(|| {
                currencies
                    .currencies
                    .values()
                    .min_by(|a, b| a.exchange_rate.partial_cmp(&b.exchange_rate).unwrap())
                    .map(|currency| currency.symbol.clone())
            })
            .unwrap_or_else(|| "c".into());
        currencies
    }

    /// Add a new currency
    pub fn add_currency(&mut self, currency: Currency) -> Result<(), String> {
        self.add_currency_with_id(currency.symbol.clone(), currency)
    }

    pub fn add_currency_with_id(
        &mut self,
        id: impl Into<String>,
        currency: Currency,
    ) -> Result<(), String> {
        if self.currencies.contains_key(&currency.symbol) {
            return Err(format!("Currency {} already exists.", currency.symbol));
        }
        let id = id.into();
        self.currency_ids.insert(id, currency.symbol.clone());
        self.currencies.insert(currency.symbol.clone(), currency);
        Ok(())
    }

    pub fn symbol_for(&self, id_or_symbol: &str) -> Option<String> {
        let key = id_or_symbol.trim();
        if key.is_empty() {
            return None;
        }
        if self.currencies.contains_key(key) {
            return Some(key.to_string());
        }
        self.currency_ids.get(key).cloned()
    }

    /// Get a reference to a currency by symbol
    pub fn get_currency(&self, symbol: &str) -> Option<&Currency> {
        self.currencies.get(symbol)
    }

    pub fn convert_to_base_by_id_or_symbol(
        &self,
        amount: i64,
        id_or_symbol: &str,
    ) -> Result<i64, String> {
        let symbol = self
            .symbol_for(id_or_symbol)
            .ok_or_else(|| format!("Currency {} not found.", id_or_symbol))?;
        self.convert_to_base(amount, &symbol)
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

    pub fn format_base_amount(&self, base_amount: i64) -> String {
        let mut amount = base_amount.max(0);
        if amount == 0 {
            return self
                .currencies
                .get(&self.base_currency)
                .map(|currency| format!("0{}", currency.symbol))
                .unwrap_or_else(|| "0".into());
        }

        let mut sorted = self.currencies.values().collect::<Vec<_>>();
        sorted.sort_by(|a, b| b.exchange_rate.partial_cmp(&a.exchange_rate).unwrap());

        let mut parts = Vec::new();
        for currency in sorted {
            let unit = currency.exchange_rate.round().max(1.0) as i64;
            if amount >= unit {
                let count = amount / unit;
                amount %= unit;
                parts.push(format!("{}{}", count, currency.symbol));
            }
        }
        if parts.is_empty() {
            format!("0{}", self.base_currency)
        } else {
            parts.join(" ")
        }
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

        let base_amount = currencies.convert_to_base(amount, symbol)?;
        let current = self.get_balance(currencies);
        self.balances.clear();
        self.balances
            .insert(currencies.base_currency.clone(), current + base_amount);

        Ok(())
    }

    /// Spend funds in the base amount.
    pub fn spend(&mut self, base_amount: i64, currencies: &Currencies) -> Result<(), String> {
        if base_amount < 0 {
            return Err("Cannot spend a negative amount.".to_string());
        }
        let current = self.get_balance(currencies);
        if current < base_amount {
            return Err("Insufficient funds.".to_string());
        }
        self.balances.clear();
        self.balances
            .insert(currencies.base_currency.clone(), current - base_amount);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_formats_ruleset_economy() {
        let rules = r#"
            [economy]
            base = "copper"

            [economy.currencies.copper]
            name = "Copper"
            symbol = "c"
            value = 1

            [economy.currencies.silver]
            name = "Silver"
            symbol = "s"
            value = 10

            [economy.currencies.gold]
            name = "Gold"
            symbol = "g"
            value = 100
        "#;
        let currencies = Currencies::from_rules_source(rules);
        assert_eq!(currencies.base_currency, "c");
        assert_eq!(
            currencies.convert_to_base_by_id_or_symbol(2, "gold"),
            Ok(200)
        );
        assert_eq!(currencies.format_base_amount(125), "1g 2s 5c");
        assert_eq!(currencies.format_base_amount(0), "0c");
    }

    #[test]
    fn wallet_stores_and_spends_base_units() {
        let currencies = Currencies::official_default();
        let mut wallet = Wallet::default();

        wallet.add("g", 1, &currencies).unwrap();
        wallet.add("s", 2, &currencies).unwrap();
        wallet.add_base_currency(5, &currencies).unwrap();

        assert_eq!(wallet.get_balance(&currencies), 125);
        assert_eq!(wallet.balances.get("c"), Some(&125));

        wallet.spend(37, &currencies).unwrap();
        assert_eq!(wallet.get_balance(&currencies), 88);
        assert_eq!(
            currencies.format_base_amount(wallet.get_balance(&currencies)),
            "8s 8c"
        );
    }
}
