# Setup

### Installations

Install Rust and GMP arithmetic library:

```jsx
brew install rust
```

```jsx
brew install gmp
```

### Build

In `core-apps/signing` folder:

```jsx
cargo build --release --all-targets
```

MacOS has a [known issue](https://github.com/ZenGo-X/multi-party-ecdsa/issues/66) where `rustc` has trouble locating the `gmp` library. You may see something similar to the following error:

```jsx
ld: library not found for -lgmp
clang: error: linker command failed with exit code 1
```

If this happens, link the library manually by running:

```jsx
export LIBRARY_PATH=$LIBRARY_PATH:/opt/homebrew/lib
export INCLUDE_PATH=$INCLUDE_PATH:/opt/homebrew/include
```

# Start Server

`cd` into `core-apps/target/release/`. Start the server:

```jsx
./gg20_sm_manager
```

# KeyGen

We will generate keys for a scheme that has 3 separate parties and a threshold of 1 party.

In 3 separate terminal windows:

```jsx
./gg20_keygen -t 1 -n 3 -i 1 --output local-share1.json
```

```jsx
./gg20_keygen -t 1 -n 3 -i 2 --output local-share2.json
```

```jsx
./gg20_keygen -t 1 -n 3 -i 3 --output local-share3.json
```

The `local-share` components are the secret keys.

# Signing

We will sign the message `“hello”` by passing in the indices of the parties who attended the signing (`-p 1,2`).

```jsx
./gg20_signing -p 1,2 -d "hello" -l local-share1.json
```

```jsx
./gg20_signing -p 1,2 -d "hello" -l local-share2.json
```

The output will be the joint signature. It should look something like this:

```jsx
{
   "r":{
      "curve":"secp256k1",
      "scalar":[
         190,
         83,
         147,
         97,
         147,
         24,
         171,
         144,
         225,
         140,
         23,
         29,
         224,
         199,
         108,
         179,
         0,
         20,
         105,
         197,
         99,
         173,
         52,
         136,
         166,
         196,
         94,
         151,
         149,
         223,
         65,
         156
      ]
   },
   "s":{
      "curve":"secp256k1",
      "scalar":[
         37,
         27,
         175,
         251,
         42,
         109,
         130,
         42,
         185,
         37,
         121,
         21,
         159,
         214,
         217,
         8,
         203,
         171,
         149,
         109,
         225,
         71,
         100,
         192,
         182,
         251,
         82,
         12,
         103,
         249,
         111,
         4
      ]
   },
   "recid":0
}
```
