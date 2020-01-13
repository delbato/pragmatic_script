# pragmatic\_script

_A pragmatic scripting language_

### <i>! This is all very heavily WIP. Does not work properly yet !</i>

## What is this?

PragmaticScript is a small scripting language i implemented myself  
as a learning exercise, and because i wanted a language where i can control all  
design variables myself.

## Why?

For no real reason, except the following:
* As a learning exercise
* To make it easily embeddable in Rust
* As an alternative to Python for shell scripting (as i detest Python)
* For the lolz 

## Goals

Some of my goals with this are:
* [x] Should be able to run basic algorithms (eg, fibonacci)
* [x] Should be able to call rust functions
* [ ] Should offer an easy API for embedding
* [ ] Replace languages like Python for shell scripting
* [ ] Precompilation
* [ ] Be reasonably fast

## Current status/TODO

* [x] Handles integer arithmetic
* [x] Handles float arithmetics
* [x] Structure scripts into (sub)modules
* [x] Supports calling functions
* [x] Supports string handling
* [x] Supports simple conditionals (if without else)
* [ ] Supports complex conditionals (if/elseif/else, switch/case...)
* [ ] PARTIAL: Supports loops (loop, while, for etc...) (see FN#1)
* [ ] PARTIAL: Supports custom types (Containers) (see FN#2)
* [x] Supports calling rust functions (see FN#3)
* [ ] Supports embedding/exposing rust native types

## Design

This is what a simple .pgs script could look like:  
```
mod: inner_module {
    fn: add(lhs: int, rhs: int) ~ int {
        return lhs + rhs;
    }
}

// This is basically a struct.
cont: Vector {
    x: float;
    y: float;
}

// Struct implementation
impl: Vector {
    fn: length() ~ float {
        return float::sqrt((x*x)+(y*y));
    }
}

import inner_module::add = add_fn;

fn: main() ~ int {
    var lhs: int = 1;
    var rhs: int = 2;
    return add_fn(lhs, rhs);
}
```

## Footnotes
1. Currently implementing - "while" and "loop" seem to compile fine, but havent been tested to run
2. Currently implementing
3. Incomplete: I want to change the low level interface of calling foreign functions
