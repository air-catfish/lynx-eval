# lynx-eval

Быстрый эвалюатор 5–7 карт и эквити «рука против руки». Rust, без зависимостей и
`unsafe`, Apache-2.0. Не солвер, не GTO и не диапазоны.

```rust
use lynx_eval::{Card, equity_vs_hand};

let hero = ["As".parse::<Card>().unwrap(), "Ah".parse().unwrap()];
let villain = ["Ks".parse().unwrap(), "Kd".parse().unwrap()];
let flop = ["2c".parse().unwrap(), "7d".parse().unwrap(), "9h".parse().unwrap()];
let equity = equity_vs_hand(hero, villain, &flop, &[]).unwrap();
println!("{:.1}%", equity.hero_equity() * 100.0);
```

Тесты: `cargo test`; 7462 класса — `cargo test --release -- --ignored`.
Документация API — `cargo doc --open`.
