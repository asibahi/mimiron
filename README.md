# Mimiron

An overengineered CLI tool and Discord bot to look up Hearthstone cards. It feeds up on the official Blizzard API and therefore, in theory, always works and always has the official, and updated, data for all cards.

![An example of card lookup. Development screenshot](README/cardlookup.png)

The following is a write up about the CLI app. For the discord bot, please go to [The Bot's README.](./mimiron_bot/README.md)

## Installation

You need to have Rust installed on your system. You can then fork this repositry and install the app on your machine with.

```sh
cargo install --path ./mimiron_cli/
```

However, you need your Blizzard API credentials as environment variables under `BLIZZARD_CLIENT_ID` and `BLIZZARD_CLIENT_SECRET`.

## Usage

### Card Lookup

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

### Deck Lookup

Look up a deck:

```sh
mimiron deck AAECAa0GCOWwBKi2BJfvBO+RBeKkBf3EBc/GBcbHBRCi6AOEnwShtgSktgSWtwT52wS43AS63ASGgwXgpAW7xAW7xwX7+AW4ngbPngbRngYAAQO42QT9xAX/4QT9xAXFpQX9xAUAAA==
```

![Deck look up in terminal](README/decklookup.png)

If the deck has E.T.C., Band Manager, you can add the band members with `--addband` argument. The card names should be exact, or at least give a unique card. This gives you the updated deck code in the output.

```sh
mimiron deck --addband "Holy Maki Roll" "Melted Maker" "Anachronos" AAECAZ8FBsvEBf3EBcHGBYv+BY3+BdiBBgzJoATquQTavQTA4gSgmQXBxAXu6QWt7QWK/gXCggaOlQaGowYA 
```

![Deck image](README/addbanddecklookup.png)

This also handles Duels mode dual-class deck *mostly* correctly. (Doesn't edit the class name dynamically.)

```sh
mimiron deck AAEBAa6aBQ/6Du/3ApXNA5PQA6bvA/D2A+CLBMeyBN65BJzHBJbUBJ+kBeKkBc+eBrChBgAAAA==
```

![Deck image](README/duelsdecklookup.png)

Save an image of the deck with the `--image` flag. Defaults to your Downloads folder unless you specify `--output`.

Note: Most images are acquired from Blizzard's servers. Ommissions are filled from https://hearthstonejson.com 

```sh
mimiron deck --image AAECAa0GCOWwBKi2BJfvBO+RBeKkBf3EBc/GBcbHBRCi6AOEnwShtgSktgSWtwT52wS43AS63ASGgwXgpAW7xAW7xwX7+AW4ngbPngbRngYAAQO42QT9xAX/4QT9xAXFpQX9xAUAAA== 
```

![Deck image](README/deckimagesquare.png)


There are other formats: `groups`, `single`, `wide`, and `text`.

```sh
mimiron deck --image --format groups AAECAa0GCOWwBKi2BJfvBO+RBeKkBf3EBc/GBcbHBRCi6AOEnwShtgSktgSWtwT52wS43AS63ASGgwXgpAW7xAW7xwX7+AW4ngbPngbRngYAAQO42QT9xAX/4QT9xAXFpQX9xAUAAA== 
```
This is similar to [Hearthstone Top Decks'](https://www.hearthstonetopdecks.com) format.
![Deck image](README/deckimage.png)

```sh
mimiron deck --image --format single AAECAa0GCOWwBKi2BJfvBO+RBeKkBf3EBc/GBcbHBRCi6AOEnwShtgSktgSWtwT52wS43AS63ASGgwXgpAW7xAW7xwX7+AW4ngbPngbRngYAAQO42QT9xAX/4QT9xAXFpQX9xAUAAA== 
```
Image is rotated so it doesn't distort this page so much:
![Deck image](README/deckimagesingle.png)

```sh
mimiron deck --image --format wide AAECAa0GCOWwBKi2BJfvBO+RBeKkBf3EBc/GBcbHBRCi6AOEnwShtgSktgSWtwT52wS43AS63ASGgwXgpAW7xAW7xwX7+AW4ngbPngbRngYAAQO42QT9xAX/4QT9xAXFpQX9xAUAAA== 
```

![Deck image](README/deckimagewide.png)

```sh
mimiron deck --image --format text AAECAa0GCOWwBKi2BJfvBO+RBeKkBf3EBc/GBcbHBRCi6AOEnwShtgSktgSWtwT52wS43AS63ASGgwXgpAW7xAW7xwX7+AW4ngbPngbRngYAAQO42QT9xAX/4QT9xAXFpQX9xAUAAA== 
```

![Deck image](README/deckimagetext.png)

Compare two decks with `--comp`:

```sh
mimiron deck --comp AAECAa0GCoSfBOWwBKi2BP/hBJfvBO+RBeKkBf3EBc/GBc2eBg+i6AOhtgSktgSWtwT52wS43AS63ASGgwXgpAW7xAW7xwX7+AW4ngbPngbRngYA AAECAa0GCKG2BKi2BOy6BO+RBc/GBc/2Bdj2Ba//BQv52wS43AS63ASGgwWkkQXgpAW7xwWm8QXt9wXjgAa4ngYA
```

![Deck comparison in terminal](README/deckcompare.png)

### Battlegrounds Lookup

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

- get deck code from list of cards. This could work like the `--addband` flag, but this makes a lot of requests for one command, more than I'd like.
- Duels support. However, Duels info seems to be incomplete in the API.
- If you would like Mercenaries support, let me know.

## License

MIT license. Don't care what you do with this, but give credit.

## Contribute

Suggestions and help welcome.

Please play around with it, abuse it, and let me know you things should work.
