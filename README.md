# lynx-eval

Быстрый эвалюатор покерных рук на 5–7 карт и **точное** эквити «рука против руки».
Ноль зависимостей, ноль `unsafe`, только карты и числа.

```rust
use lynx_eval::{Card, equity_vs_hand, evaluate};

let hero = ["As".parse::<Card>().unwrap(), "Ah".parse().unwrap()];
let villain = ["Ks".parse().unwrap(), "Kd".parse().unwrap()];
let flop = ["2c".parse().unwrap(), "7d".parse().unwrap(), "9h".parse().unwrap()];

let value = evaluate(&["As".parse().unwrap(), "Ks".parse().unwrap(), "Qs".parse().unwrap(),
                       "Js".parse().unwrap(), "Ts".parse().unwrap()]).unwrap();
println!("{}", value.category() as u8); // 8 = StraightFlush

let equity = equity_vs_hand(hero, villain, &flop, &[]).unwrap();
println!("hero {:.2}%", equity.hero_equity() * 100.0); // 91.62
```

## Чего это НЕ

- Не солвер и не GTO-советник: никаких диапазонов, частот и «правильных» решений.
- Не оценка решений и не статистика сессий.
- Не поддержка омахи, короткой колоды и джокеров.

Эквити считается только против **известной** руки. Против диапазона — не в этой
версии; выдумывать диапазон из воздуха крейт не станет.

## API

| Что | Сигнатура |
|---|---|
| Разбор карты | `Card::parse("As") -> Result<Card, CardError>`, регистр не важен |
| Оценка руки | `evaluate(&[Card]) -> Result<HandValue, EvalError>`, 5–7 карт |
| Лучшая пятёрка | `best_five(&[Card]) -> Result<[Card; 5], EvalError>` |
| Разбор силы | `describe(HandValue) -> HandDescription` (категория + ранги, без строк) |
| Точное эквити | `equity_vs_hand(hero, villain, board, dead) -> Result<Equity, EvalError>` |
| Монте-Карло | `equity_vs_hand_mc(..., samples, seed) -> Result<Equity, EvalError>` |

`HandValue` — упакованный `u32`, сравнимый через `Ord`: категория, затем кикеры.
Числовое представление непрозрачно и не является стабильным контрактом.

`Equity` нормировано: `wins + losses + ties == 1`, `hero_equity() == wins + ties / 2`.
В Монте-Карло `exact == false` и видно `samples`.

Невалидный вход (меньше 5 или больше 7 карт, дубль карты, борд из 1–2 карт,
`dead`, пересекающийся с рукой, `samples == 0`) возвращает `EvalError` —
никаких паник и «тихо неверных» чисел.

## Точность и скорость

Точный перебор перебирает все борды: префлоп — `C(48,5) = 1 712 304` борда и две
оценки на борд; флоп — 990, терн — 44, ривер — 1.

Замеры на одной машине (Rust 1.100, release-профиль: `opt-level = 3`,
`lto = "fat"`, `codegen-units = 1`):

| Операция | Время |
|---|---|
| `evaluate` 7 карт | **37.6 нс** на руку |
| точный префлоп AA vs KK | **129 мс** (1 712 304 борда) |
| Монте-Карло 200 000 сэмплов | **29 мс** |
| точный флоп | 69 мкс |

Числа — из `cargo test -p lynx-eval --release --test perf -- --ignored --nocapture`;
на другой машине они будут другими, важна методика, а не абсолют.

## Тесты

```powershell
# быстрые тесты
cargo test -p lynx-eval

# тяжёлая проверка: все C(52,5) = 2 598 960 пятёрок дают ровно 7462 класса
cargo test -p lynx-eval --release -- --ignored

# замеры
cargo test -p lynx-eval --release --test perf -- --ignored --nocapture

cargo clippy -p lynx-eval --all-targets -- -D warnings
cargo fmt -p lynx-eval --check
```

Тест «7462 класса» — главный: он проверяет, что эвалюатор действительно
различает руки, а не просто печатает категорию. Мэтчапы сверены с независимыми
публичными переборами: AA vs KK 81.95 / 82.64 / 81.26% по конфигурациям мастей,
AK♣K♣ vs 2♦2♥ — 50.084%.

## Совместимость

- MSRV: Rust **1.85** (edition 2024).
- Лицензия: **Apache-2.0**, см. `LICENSE`.
- Не опубликован на crates.io; публикация — отдельное решение владельца.

## Вклад

Пока это закрытый проект, публичный вклад не принимается. Issues и предложения —
через владельца репозитория.
