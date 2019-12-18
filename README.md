# PragmaticScript

A pragmatic scripting language

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
* [ ] Should be able to run basic algorithms (eg, fibonacci)
* [ ] Should be able to call rust functions
* [ ] Should offer an easy API for embedding
* [ ] Replace languages like Python for shell scripting
* [ ] Precompilation
* [ ] Be reasonably fast

## Current status/TODO

* [x] Handles integer arithmetic
* [x] Structure scripts into (sub)modules
* [ ] PARTIAL: Supports calling functions (see FN#1)
* [ ] PARTIAL: Handles float arithmetics (see FN#2)
* [ ] Supports string handling
* [x] Supports simple conditionals (if without else)
* [ ] Supports complex conditionals (if/elseif/else, switch/case...)
* [ ] Supports loops (loop, while, for etc...)
* [ ] PARTIAL: Supports custom types (Containers) (see FN#3)
* [ ] Supports calling rust functions
* [ ] Supports embedding/exposing rust native types

## Footnotes

1. The interpreter already handles function calls correctly, but  
    the compiler doesnt fully support call compilation yet (see unit tests)
2. Float arithmetics are "disabled" right now - making them work would be trivial
3. Currently implementing.

## Design

This is what a simple .pgs script could look like:  
```
mod: inner_module {
    fn: add(lhs: int, rhs: int) ~ int {
        return lhs + rhs;
    }
}

// This is basically a struct.
cont: Point {
    x: float;
    y: float;
}

import inner_module::add = add_fn;

fn: main() {
    var:int lhs = 1;
    var:int rhs = 2;
    return add_fn(lhs, rhs);
}
```
