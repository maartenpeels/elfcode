# elfcode

Solve [Advent of Code](https://adventofcode.com) in Rust with one small trait impl per day. A `cargo elfcode` command fetches the puzzle and your input, runs and times both parts, and submits answers. Local checks stop answers that cannot be right from ever being sent.

```text
$ cargo elfcode run 3 --submit 1
2026 · day 03
  parse      112.4µs
  part 1     161          18.2µs
  submit     correct; puzzle.md now includes part 2
```

## Setup

Install the command once:

```sh
cargo install cargo-elfcode
```

Create a crate for your solutions. All years live in the same crate.

```sh
cargo new advent-of-code
cd advent-of-code
cargo add elfcode
cargo add --build elfcode-build
```

Add a `build.rs` next to `Cargo.toml`:

```rust
fn main() {
    elfcode_build::generate();
}
```

Replace `src/main.rs` with this single line:

```rust
elfcode::main!();
```

Log in with the `session` cookie from adventofcode.com. You can find it in your browser's developer tools once you are logged in. It is checked against the site and stored in your user config directory, outside the repository.

```sh
cargo elfcode login <session-cookie>
```

## Solving a day

```sh
cargo elfcode scaffold 3
```

This creates four files:

| File | Contents |
|---|---|
| `src/y2026/day03.rs` | The solution, with both parts unsolved |
| `data/2026/03/examples.toml` | Your examples, empty apart from a format template |
| `data/2026/03/puzzle.md` | The puzzle text as markdown |
| `data/2026/03/input.txt` | Your puzzle input |

Existing files are never overwritten. Before a puzzle unlocks, at midnight US Eastern, only the first two are created; `run` downloads the input later.

A day is a type named after the day that implements `Solution`:

```rust
use elfcode::prelude::*;

pub struct Day03;

impl Solution for Day03 {
    type Input<'a> = Vec<&'a str>;

    fn parse(raw: &str) -> Result<Self::Input<'_>> {
        Ok(raw.lines().collect())
    }

    fn part1(input: &Self::Input<'_>) -> Result<impl Answer> {
        Ok(input.len())
    }

    fn part2(_input: &Self::Input<'_>) -> Result<impl Answer> {
        Ok(Unsolved)
    }
}
```

- `parse` runs once and both parts borrow its result, so work the parts share belongs there. The parsed input may borrow from the raw text.
- A part returns any integer type, `&str` or `String`.
- `Ok(Unsolved)` and `todo!()` are reported as "not implemented". Any other error or panic is reported without stopping the run.
- `Result`, `Context`, `bail!` and `ensure!` come from [anyhow](https://docs.rs/anyhow) through the prelude.

Files must follow the naming exactly: `src/y{YYYY}/day{DD}.rs` with a `Day{DD}` type. The build fails with a message naming the file when one does not. A day may keep submodules in `src/y{YYYY}/day{DD}/`.

## Running

```sh
cargo elfcode run 3                # both parts, latest event
cargo elfcode run 3 --year 2024    # another year
cargo elfcode run 3 --part 2       # one part
cargo elfcode run 3 --debug        # unoptimized build; timings are unreliable
```

Runs use a release build by default. Parse time is shown separately from each part. Anything your solution prints is passed through to the terminal.

Without `--year`, the latest event is used: this year's from December 1, last year's before that.

## Examples as tests

Copy the examples from the puzzle text into `examples.toml`:

```toml
[[example]]
input = '''
1 2 3
4 5 6
'''
part1 = 21
part2 = "abc"
```

Each example becomes one test per part it gives an answer for, run by plain `cargo test`:

```text
test y2026::day03_examples::example_1::part1 ... ok
```

- `cargo test day03` runs one day.
- A wrong answer shows what was expected and what came out.
- Use `'''` literal strings, so the input keeps its whitespace exactly.
- An answer is text or a whole number. An example without any answer is a build error.

## Submitting

```sh
cargo elfcode run 3 --submit 1
```

This runs part 1 and sends its answer.

Nothing is sent when it cannot be right:

- the part is already solved;
- the same answer was already sent;
- an earlier answer that was too high or too low rules it out;
- the site's wait after a wrong answer is still running;
- part 2 is sent before part 1 is solved. If you solved part 1 elsewhere, submitting part 1 again records that.

Every answer and the site's verdict are kept in `data/{YYYY}/{DD}/submissions.toml`. Once part 1 is solved, `puzzle.md` is downloaded again to include part 2.

## Being a good citizen

Advent of Code asks that puzzle text and inputs are not published, and that tools go easy on the servers:

- `puzzle.md` and `input.txt` are added to your `.gitignore` before they are first written. Your examples and submission history stay in version control.
- Everything is downloaded once and then read from disk. Nothing is requested before a puzzle unlocks.
- Requests identify themselves with this project's repository as contact.

## Versions

Keep `cargo-elfcode`, `elfcode` and `elfcode-build` on the same version. They talk to each other through generated code and a small protocol, and say so when they do not match. The minimum supported Rust version is 1.88.

## License

Licensed under [MIT license](LICENSE).
