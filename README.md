# Mimiron

An overengineered CLI tool to look up Hearthstone cards. It feeds up on the official Blizzard API and therefore, in theory, always works and always has the official, and updated, data for all cards.

![An example of card lookup. Development screenshot](README/cardlookup.png)

## Installation

You need to have Rust installed on your system. You can then install this app on your machine with:

```sh
cargo install --git https://github.com/asibahi/mimiron
```

## Usage

You can look up a card with:

```sh
mimiron card Ragnaros
```

If the text you're looking up includes spaces or apostrophes they need to be in quotation marks (or the shell trips up), or you can escape them.

```sh
mimiron card "Al'Akir"
```

```sh
mimiron card Ace\ Hunter
```

You can also search in card text boxes (which is the default mode for Blizzard's API).

```sh
mimiron card ragnaros -t
```

![Card text box search](README/cardtextlookup.png)

You can look up a deck with:

```
mimiron deck AAECAa0GCOWwBKi2BJfvBO+RBeKkBf3EBc/GBcbHBRCi6AOEnwShtgSktgSWtwT52wS43AS63ASGgwXgpAW7xAW7xwX7+AW4ngbPngbRngYAAQO42QT9xAX/4QT9xAXFpQX9xAUAAA==

```

![Deck look up in terminal](README/decklookup.png)

You can also compare two decks.

```sh
mimiron deck -c AAECAa0GCoSfBOWwBKi2BP/hBJfvBO+RBeKkBf3EBc/GBc2eBg+i6AOhtgSktgSWtwT52wS43AS63ASGgwXgpAW7xAW7xwX7+AW4ngbPngbRngYA AAECAa0GCKG2BKi2BOy6BO+RBc/GBc/2Bdj2Ba//BQv52wS43AS63ASGgwWkkQXgpAW7xwWm8QXt9wXjgAa4ngYA
```

![Deck comparison in terminal](README/deckcompare.png)

You can look up Battlegrounds minions and Heroes

```sh
mimiron bg "N'zoth"
```

![Battleground lookup](README/bglookup.png)

Please play around with it, abuse it, and let me know you things should work.