# ruruby ![ruruby](https://github.com/sisshiki1969/ruruby/workflows/Rust/badge.svg)![codecov](https://codecov.io/gh/sisshiki1969/ruruby/branch/master/graph/badge.svg)

An alternative Ruby implementation by Rust.

## Features

- Purely implemented with Rust.
- No dependency on any other Ruby implementation such as CRuby(MRI), mruby, .. etc.
- Hand-written original parser.
- Virtual machine execution.
- :ribbon: Simple mark & sweep garbage collector is implemented. 

## Related article (sorry, currently only in Japanese)

[Qiita: Rust でつくる（つくれるかもしれない）Ruby](https://qiita.com/sisshiki1969/items/3d25aa81a376eee2e7c2)

## Implementation status

[See Wiki.](https://github.com/sisshiki1969/ruruby/wiki/Implementation-status)

## Benchmarks

[See Wiki.](https://github.com/sisshiki1969/ruruby/wiki/Benchmarks)  
You can see the results of optcarrot benchmark for ruruby and other Ruby implementations [here](https://github.com/mame/optcarrot/blob/master/doc/benchmark.md).

## How to run ruruby

To build ruruby, You'll need installation of Rust.
Please be aware that **only nightly version of Rust works** for ruruby.

To run ruby program file on ruruby,

```sh
% cargo run tests/sample.rb
```

or

```sh
% cargo run --release -- tests/sample.rb
```

You can launch irb-like interactive shell, omitting file name.

```sh
% cargo run
```

There are some useful options for analysis and development. Use `feature` flag of cargo.

### `trace` option: bytecode trace execution

```sh
% cargo run --features trace -- tests/sample.rb
   Compiling ruruby v0.1.0
    Finished dev [unoptimized + debuginfo] target(s) in 1.83s
     Running `target/debug/ruruby tests/sample.rb`
---> MethodRef(198)
   0:PUSH_STRING     stack:0
   5:SET_LOCAL       stack:1
   e:PUSH_STRING     stack:0
  13:PUSH_STRING     stack:1
  18:CONCAT_STR      stack:2
  19:GET_LOCAL       stack:1
  22:TO_S            stack:2
  23:CONCAT_STR      stack:2
  24:PUSH_STRING     stack:1
  29:CONCAT_STR      stack:2
  2a:SEND_SELF       stack:1
Hello world!
  3b:END             stack:1
<--- Ok(nil)
```

### `emit-iseq` option: dump bytecode

```sh
% cargo run --features emit-iseq -- tests/sample.rb
   Compiling ruruby v0.1.0
    Finished dev [unoptimized + debuginfo] target(s) in 6.72s
     Running `target/debug/ruruby tests/sample.rb`
-----------------------------------------
MethodRef(200)
local var(0): w 
block: None
  00000 PUSH_STRING 182
  00005 SET_LOCAL 'w' outer:0 LvarId:0
  0000e PUSH_STRING 183
  00013 PUSH_STRING 184
  00018 CONCAT_STR
  00019 GET_LOCAL 'w' outer:0 LvarId:0
  00022 TO_S
  00023 CONCAT_STR
  00024 PUSH_STRING 185
  00029 CONCAT_STR
  0002a OPT_SEND_SELF 'puts' 1 items
  00035 END
Hello world!
```

### `perf` option: performance analysis per VM instruction

```sh
% cargo run --release --features perf -- tests/app_mandelbrot.rb > /dev/null
    Finished release [optimized] target(s) in 0.50s
     Running `target/release/ruruby tests/app_mandelbrot.rb`
Performance analysis for Inst:
------------------------------------------
Inst name         count    %time     nsec
                                    /inst
------------------------------------------
PUSH_FIXNUM        680K     0.14       29
PUSH_FLONUM        960K     0.20       28
PUSH_TRUE           96K     0.02       30
PUSH_FALSE         160K     0.03       30
PUSH_NIL            96K     0.02       28
PUSH_STRING           4     0.00      250
PUSH_SYMBOL           3     0.00      166
ADD                 19M     6.81       50
SUB               4161K     1.10       37
MUL                 27M    10.38       53
DIV                320K     0.09       40
EQ                 300K     0.08       39
GT                7907K     2.30       41
SHL                160K     0.04       37
BIT_OR             160K     0.04       33
ADDI              4065K     0.95       33
SUBI               140K     0.04       38
SET_LOCAL         8843K     1.92       30
GET_LOCAL           64M    13.54       29
GET_CONST           15M     5.65       50
SET_CONST             2     0.00      100
GET_IVAR            38M    10.86       39
SET_IVAR            16M    10.72       94
OPT_SEND            34M    16.62       67
OPT_SEND_SELF       20K     0.01       84
CREATE_RANGE          1     0.00      100
POP                116K     0.03       31
DUP               8002K     1.70       29
CONCAT_STR            5     0.00      200
TO_S                  2     0.00      550
DEF_CLASS             1     0.00     1700
DEF_METHOD            4     0.00      175
JMP               4182K     0.82       27
JMP_IF_FALSE      8367K     1.86       31
END                 19M     3.97       28
GC *                741     1.29   246414
EXTERN **         8042K     8.76      153
------------------------------------------
```

Instruction name, total execution count, percentage in the total execution time, and 
execution time per single instruction are shown.  
\* `GC` means garbage collection.  
\** `EXTERN` means exectution of native methods.  
