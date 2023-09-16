# Mimiron

An overengineered CLI tool to look up Hearthstone cards. It feeds up on the official Blizzard API and therefore, in theory, always works and always has the official, and updated, data for all cards.

![An example of card lookup. Development screenshot](README/cardlookup.png)

## Installation

You need to have Rust installed on your system. You can then fork this repositry and install the app on your machine with.

```sh
cargo install --path .
```

or you can immediately install from GitHub with

```sh
cargo install --git https://github.com/asibahi/mimiron
```

However, you need your Blizzard API credentials as environment variables under `BLIZZARD_CLIENT_ID` and `BLIZZARD_CLIENT_SECRET`.

## Usage

Look up a card:

```sh
mimiron card ragnaros
```

Add image links with `--image` (works with Battlegrounds look ups as well):

```sh
mimiron card ragnaros --image
```

![Card look up with image links](README/cardimagelookup.png)

If the text you're looking up includes spaces or apostrophes they need to be in quotation marks (or the shell trips up), or you can escape them:

```sh
mimiron card "Al'Akir"
```

```sh
mimiron card Ace\ Hunter
```

Include card text boxes in search (which is the default mode for Blizzard's API):

```sh
mimiron card ragnaros --text
```

![Card text box search](README/cardtextlookup.png)

Look up a deck:

```
mimiron deck AAECAa0GCOWwBKi2BJfvBO+RBeKkBf3EBc/GBcbHBRCi6AOEnwShtgSktgSWtwT52wS43AS63ASGgwXgpAW7xAW7xwX7+AW4ngbPngbRngYAAQO42QT9xAX/4QT9xAXFpQX9xAUAAA==
```

![Deck look up in terminal](README/decklookup.png)

Save an image of the deck with the `--image` flag. Defaults to your Downloads folder unless you specify `--output`:

```
mimiron deck AAECAa0GCOWwBKi2BJfvBO+RBeKkBf3EBc/GBcbHBRCi6AOEnwShtgSktgSWtwT52wS43AS63ASGgwXgpAW7xAW7xwX7+AW4ngbPngbRngYAAQO42QT9xAX/4QT9xAXFpQX9xAUAAA== --image
```

![Deck image](README/deckimage.png)

(Yes, it looks like the [Hearthstone Top Decks](https://www.hearthstonetopdecks.com) format. I am a fan.)

There are a couple more compact formats: `--single` and `--wide`.

```
mimiron deck AAECAa0GCOWwBKi2BJfvBO+RBeKkBf3EBc/GBcbHBRCi6AOEnwShtgSktgSWtwT52wS43AS63ASGgwXgpAW7xAW7xwX7+AW4ngbPngbRngYAAQO42QT9xAX/4QT9xAXFpQX9xAUAAA== --image --wide
```

![Deck image](README/deckimagewide.png)

```
mimiron deck AAECAa0GCOWwBKi2BJfvBO+RBeKkBf3EBc/GBcbHBRCi6AOEnwShtgSktgSWtwT52wS43AS63ASGgwXgpAW7xAW7xwX7+AW4ngbPngbRngYAAQO42QT9xAX/4QT9xAXFpQX9xAUAAA== --image
```

![Deck image](README/deckimage.png)

Compare two decks:

```sh
mimiron deck -c AAECAa0GCoSfBOWwBKi2BP/hBJfvBO+RBeKkBf3EBc/GBc2eBg+i6AOhtgSktgSWtwT52wS43AS63ASGgwXgpAW7xAW7xwX7+AW4ngbPngbRngYA AAECAa0GCKG2BKi2BOy6BO+RBc/GBc/2Bdj2Ba//BQv52wS43AS63ASGgwWkkQXgpAW7xwWm8QXt9wXjgAa4ngYA
```

![Deck comparison in terminal](README/deckcompare.png)

Look up Battlegrounds minions and Heroes:

```sh
mimiron bg elise
```

![Battleground lookup](README/bglookup.png)

Look up by tier and/or type:

```sh
mimiron bg --tier 1 --type beast
```

![Battleground lookup](README/bgtiertypelookup.png)

## Roadmap

Nothing in particular, however some ideas are:

- add the sideboard cards to decks that should have them but miss them, and give the new code. Not sure how the command would look like.
- get deck code from list of cards. Also not sure how this should look like.
- Duels support. However, Duels info seems to be incomplete in the API.

## License

MIT license. Don't care what you do with this, but give credit.

## Contribute

Suggestions and help welcome.

Please play around with it, abuse it, and let me know you things should work.