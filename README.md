# lynx-eval

Быстрый эвалюатор 5–7 карт, эквити «рука против руки», эквити против диапазона
и диапазон против диапазона с весами и мёртвыми картами. Rust, без зависимостей
и `unsafe`, Apache-2.0. Не солвер и не GTO.

```rust
use lynx_eval::{Card, Range, equity_range_vs_range, equity_vs_hand, equity_vs_range};

let hero = ["As".parse::<Card>().unwrap(), "Ah".parse().unwrap()];
let villain = ["Ks".parse().unwrap(), "Kd".parse().unwrap()];
let flop = ["2c".parse().unwrap(), "7d".parse().unwrap(), "9h".parse().unwrap()];
let equity = equity_vs_hand(hero, villain, &flop, &[]).unwrap();
println!("{:.1}%", equity.hero_equity() * 100.0);

let range = Range::parse("22+, A2s+, KQo:0.8").unwrap();
let range_equity = equity_vs_range(hero, &flop, &range, &[]).unwrap();
println!("{:.1}% против диапазона", range_equity.hero_equity() * 100.0);

// Диапазон против диапазона: только совместимые пары комбо.
let hero_range = Range::parse("QQ+, AKs").unwrap();
let villain_range = Range::parse("JJ+, AQs+").unwrap();
let duel = equity_range_vs_range(&hero_range, &villain_range, &flop, &[]).unwrap();
println!("{:.1}% / {:.1}%", duel.hero_equity * 100.0, duel.villain_equity * 100.0);
```

Точный перебор — на флопе/тёрне/ривере, пока работа укладывается в
`EXACT_COMPARISON_BUDGET` (5 млн сравнений «пара комбо × ранаут»). Иначе —
`equity_vs_range_mc` / `equity_range_vs_range_mc` с `samples`, `seed` и
`exact: false`. `dead` — известные мёртвые карты: они выкидываются из колоды и
из комбо обеих сторон.

Тесты: `cargo test`; 7462 класса — `cargo test --release -- --ignored`.
Документация API — `cargo doc --open`; пример — `cargo run --example equity_range`.
