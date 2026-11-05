//! In-Memory Level-2 Order Book.
//! Implements an institutional Price-Time Priority matching queue:
//! - Bids sorted descending (highest buyer has top priority).
//! - Asks sorted ascending (lowest seller has top priority).
//! - Level-2 market depth, spread, and mid-price calculations.

use std::collections::{BTreeMap, VecDeque};
use crate::models::account::UserId;
use crate::models::order::{OrderId, OrderSide};
use crate::types::Money;

/// A resting limit order residing inside the order book queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookOrder {
    pub order_id: OrderId,
    pub user_id: UserId,
    pub qty: u32,
    pub price: Money,
    pub timestamp_ns: u64,
}

/// The Level-2 Limit Order Book for a single instrument (e.g. "RELIANCE.NS").
#[derive(Debug, Clone, Default)]
pub struct OrderBook {
    pub symbol: String,
    /// Bids stored in BTreeMap (lowest to highest price).
    /// Best Bid is at the highest key (`bids.iter().next_back()`).
    bids: BTreeMap<Money, VecDeque<BookOrder>>,
    /// Asks stored in BTreeMap (lowest to highest price).
    /// Best Ask is at the lowest key (`asks.iter().next()`).
    asks: BTreeMap<Money, VecDeque<BookOrder>>,
}

impl OrderBook {
    /// Creates a new empty order book for a given instrument.
    pub fn new(symbol: impl Into<String>) -> Self {
        OrderBook {
            symbol: symbol.into(),
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }

    /// Returns the Best Bid (highest price a buyer is willing to pay) and total quantity at that price.
    pub fn best_bid(&self) -> Option<(Money, u32)> {
        self.bids.iter().next_back().map(|(price, queue)| {
            let total_qty: u32 = queue.iter().map(|o| o.qty).sum();
            (*price, total_qty)
        })
    }

    /// Returns the Best Ask (lowest price a seller is willing to accept) and total quantity at that price.
    pub fn best_ask(&self) -> Option<(Money, u32)> {
        self.asks.iter().next().map(|(price, queue)| {
            let total_qty: u32 = queue.iter().map(|o| o.qty).sum();
            (*price, total_qty)
        })
    }

    /// Returns the Bid-Ask Spread (`Best Ask - Best Bid`).
    pub fn spread(&self) -> Option<Money> {
        match (self.best_bid(), self.best_ask()) {
            (Some((bid, _)), Some((ask, _))) if ask >= bid => Some(ask - bid),
            _ => None,
        }
    }

    /// Returns the Mid Price (`(Best Bid + Best Ask) / 2`).
    pub fn mid_price(&self) -> Option<Money> {
        match (self.best_bid(), self.best_ask()) {
            (Some((bid, _)), Some((ask, _))) => {
                let mid_paise = (bid.to_paise() + ask.to_paise()) / 2;
                Some(Money::from_paise(mid_paise))
            }
            _ => None,
        }
    }

    /// Adds a resting limit order to the order book respecting Price-Time priority.
    pub fn add_limit_order(
        &mut self,
        side: OrderSide,
        order_id: OrderId,
        user_id: UserId,
        qty: u32,
        price: Money,
        timestamp_ns: u64,
    ) {
        if qty == 0 {
            return;
        }

        let book_order = BookOrder {
            order_id,
            user_id,
            qty,
            price,
            timestamp_ns,
        };

        match side {
            OrderSide::Buy => {
                self.bids
                    .entry(price)
                    .or_default()
                    .push_back(book_order);
            }
            OrderSide::Sell => {
                self.asks
                    .entry(price)
                    .or_default()
                    .push_back(book_order);
            }
        }
    }

    /// Cancels a resting limit order by ID across price levels.
    /// Returns true if the order was found and removed.
    pub fn cancel_order(&mut self, side: OrderSide, order_id: OrderId) -> bool {
        let book_side = match side {
            OrderSide::Buy => &mut self.bids,
            OrderSide::Sell => &mut self.asks,
        };

        let mut found_price = None;
        for (price, queue) in book_side.iter_mut() {
            if let Some(pos) = queue.iter().position(|o| o.order_id == order_id) {
                queue.remove(pos);
                if queue.is_empty() {
                    found_price = Some(*price);
                }
                break;
            }
        }

        if let Some(price) = found_price {
            book_side.remove(&price);
            true
        } else {
            // Check if queue had items but wasn't empty
            found_price.is_some() || self.contains_order_id(side, order_id)
        }
    }

    fn contains_order_id(&self, side: OrderSide, order_id: OrderId) -> bool {
        let book_side = match side {
            OrderSide::Buy => &self.bids,
            OrderSide::Sell => &self.asks,
        };
        book_side.values().any(|q| q.iter().any(|o| o.order_id == order_id))
    }

    /// Returns Level-2 aggregated market depth up to N levels.
    /// Format: `(Bids [descending], Asks [ascending])`.
    pub fn l2_depth(&self, levels: usize) -> (Vec<(Money, u32)>, Vec<(Money, u32)>) {
        let bid_depth: Vec<(Money, u32)> = self
            .bids
            .iter()
            .rev() // Highest price first
            .take(levels)
            .map(|(p, q)| (*p, q.iter().map(|o| o.qty).sum()))
            .collect();

        let ask_depth: Vec<(Money, u32)> = self
            .asks
            .iter() // Lowest price first
            .take(levels)
            .map(|(p, q)| (*p, q.iter().map(|o| o.qty).sum()))
            .collect();

        (bid_depth, ask_depth)
    }

    /// Total shares currently bid for in the order book.
    pub fn total_bid_volume(&self) -> u32 {
        self.bids.values().flat_map(|q| q.iter()).map(|o| o.qty).sum()
    }

    /// Total shares currently offered for sale in the order book.
    pub fn total_ask_volume(&self) -> u32 {
        self.asks.values().flat_map(|q| q.iter()).map(|o| o.qty).sum()
    }

    /// Accessor for matching engine (mutably access asks).
    #[allow(dead_code)]
    pub fn asks_mut(&mut self) -> &mut BTreeMap<Money, VecDeque<BookOrder>> {
        &mut self.asks
    }

    /// Accessor for matching engine (mutably access bids).
    #[allow(dead_code)]
    pub fn bids_mut(&mut self) -> &mut BTreeMap<Money, VecDeque<BookOrder>> {
        &mut self.bids
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_book_state() {
        let book = OrderBook::new("RELIANCE.NS");
        assert_eq!(book.best_bid(), None);
        assert_eq!(book.best_ask(), None);
        assert_eq!(book.spread(), None);
        assert_eq!(book.mid_price(), None);
        assert_eq!(book.total_bid_volume(), 0);
        assert_eq!(book.total_ask_volume(), 0);
    }

    #[test]
    fn test_bids_sorted_descending_by_price() {
        let mut book = OrderBook::new("TCS.NS");

        // Buyer 1: 50 shares @ Rs 3500.00 at t=1
        book.add_limit_order(OrderSide::Buy, OrderId(1), UserId(10), 50, Money::from_rupees(3500.0), 100);
        // Buyer 2: 30 shares @ Rs 3505.00 at t=2 (Higher price -> Priority!)
        book.add_limit_order(OrderSide::Buy, OrderId(2), UserId(20), 30, Money::from_rupees(3505.0), 200);
        // Buyer 3: 20 shares @ Rs 3495.00 at t=3
        book.add_limit_order(OrderSide::Buy, OrderId(3), UserId(30), 20, Money::from_rupees(3495.0), 300);

        // Best Bid must be Rs 3505.00 with 30 shares
        assert_eq!(book.best_bid(), Some((Money::from_rupees(3505.0), 30)));
        assert_eq!(book.total_bid_volume(), 100);
    }

    #[test]
    fn test_asks_sorted_ascending_by_price() {
        let mut book = OrderBook::new("TCS.NS");

        // Seller 1: 40 shares @ Rs 3515.00
        book.add_limit_order(OrderSide::Sell, OrderId(4), UserId(40), 40, Money::from_rupees(3515.0), 100);
        // Seller 2: 25 shares @ Rs 3510.00 (Lower price -> Priority!)
        book.add_limit_order(OrderSide::Sell, OrderId(5), UserId(50), 25, Money::from_rupees(3510.0), 200);

        // Best Ask must be Rs 3510.00 with 25 shares
        assert_eq!(book.best_ask(), Some((Money::from_rupees(3510.0), 25)));
        assert_eq!(book.total_ask_volume(), 65);
    }

    #[test]
    fn test_spread_and_mid_price() {
        let mut book = OrderBook::new("INFY.NS");

        book.add_limit_order(OrderSide::Buy, OrderId(1), UserId(1), 100, Money::from_rupees(1500.0), 10);
        book.add_limit_order(OrderSide::Sell, OrderId(2), UserId(2), 100, Money::from_rupees(1501.0), 20);

        // Spread = 1501.0 - 1500.0 = Rs 1.00 (100 paise)
        assert_eq!(book.spread(), Some(Money::from_rupees(1.0)));
        // Mid price = (1500 + 1501) / 2 = Rs 1500.50
        assert_eq!(book.mid_price(), Some(Money::from_rupees(1500.50)));
    }

    #[test]
    fn test_time_priority_at_same_price() {
        let mut book = OrderBook::new("HDFCBANK.NS");

        // Order A: 10 shares @ Rs 1600 at t=10
        book.add_limit_order(OrderSide::Buy, OrderId(10), UserId(1), 10, Money::from_rupees(1600.0), 10);
        // Order B: 20 shares @ Rs 1600 at t=20 (Same price, later time)
        book.add_limit_order(OrderSide::Buy, OrderId(20), UserId(2), 20, Money::from_rupees(1600.0), 20);

        // Total volume at Rs 1600 is 30 shares
        assert_eq!(book.best_bid(), Some((Money::from_rupees(1600.0), 30)));

        // Verify Order A is at front of queue (FIFO)
        let queue = book.bids.get(&Money::from_rupees(1600.0)).unwrap();
        assert_eq!(queue.front().unwrap().order_id, OrderId(10));
        assert_eq!(queue.back().unwrap().order_id, OrderId(20));
    }

    #[test]
    fn test_level2_market_depth() {
        let mut book = OrderBook::new("SBIN.NS");

        // 3 Bid levels
        book.add_limit_order(OrderSide::Buy, OrderId(1), UserId(1), 100, Money::from_rupees(800.0), 1);
        book.add_limit_order(OrderSide::Buy, OrderId(2), UserId(2), 150, Money::from_rupees(799.0), 2);
        book.add_limit_order(OrderSide::Buy, OrderId(3), UserId(3), 200, Money::from_rupees(798.0), 3);

        // 2 Ask levels
        book.add_limit_order(OrderSide::Sell, OrderId(4), UserId(4), 50, Money::from_rupees(802.0), 4);
        book.add_limit_order(OrderSide::Sell, OrderId(5), UserId(5), 80, Money::from_rupees(803.0), 5);

        let (bids, asks) = book.l2_depth(2); // Top 2 levels

        assert_eq!(bids.len(), 2);
        assert_eq!(bids[0], (Money::from_rupees(800.0), 100));
        assert_eq!(bids[1], (Money::from_rupees(799.0), 150));

        assert_eq!(asks.len(), 2);
        assert_eq!(asks[0], (Money::from_rupees(802.0), 50));
        assert_eq!(asks[1], (Money::from_rupees(803.0), 80));
    }

    #[test]
    fn test_order_cancellation() {
        let mut book = OrderBook::new("ITC.NS");

        book.add_limit_order(OrderSide::Buy, OrderId(101), UserId(1), 500, Money::from_rupees(450.0), 1);
        assert_eq!(book.total_bid_volume(), 500);

        // Cancel order 101
        book.cancel_order(OrderSide::Buy, OrderId(101));
        assert_eq!(book.total_bid_volume(), 0);
        assert_eq!(book.best_bid(), None);
    }
}
