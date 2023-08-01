# Mimiron

An overengineered CLI tool to look up Hearthstone cards. It feeds up on the official Blizzard API and therefore, in theory, always works and always has the official, and updated, data for all cards.

![An example of card lookup. Development screenshot](cardlookup.png)

## Installation

To install you need to have rust installed, and within this binary use `cargo install`.

For the time being, you need to have your own [Blizzard API](https://develop.battle.net) Client credentials as environment variables. `BLIZZARD_CLIENT_ID` and `BLIZZARD_CLIENT_SECRET`.

## Usage

You can look up a card with:

```sh
$ mimiron card Ragnaros
```

If the text you're looking up includes spaces or apostrophes they need to be in quotation marks (or the shell trips up).

```sh
$ mimiron card "Al'Akir"
```

```sh
$ mimiron card "Ace Hunter"
```

You can look up a deck with:

```
$ mimiron deck AAECAa0GCOWwBKi2BJfvBO+RBeKkBf3EBc/GBcbHBRCi6AOEnwShtgSktgSWtwT52wS43AS63ASGgwXgpAW7xAW7xwX7+AW4ngbPngbRngYAAQO42QT9xAX/4QT9xAXFpQX9xAUAAA==

```

![Deck look up in terminal](decklookup.png)

You can also compare two decks.

```sh
mimiron deck -c AAECAa0GCoSfBOWwBKi2BP/hBJfvBO+RBeKkBf3EBc/GBc2eBg+i6AOhtgSktgSWtwT52wS43AS63ASGgwXgpAW7xAW7xwX7+AW4ngbPngbRngYA AAECAa0GCKG2BKi2BOy6BO+RBc/GBc/2Bdj2Ba//BQv52wS43AS63ASGgwWkkQXgpAW7xwWm8QXt9wXjgAa4ngYA
```

Please play around with it, abuse it, and let me know you things should work.