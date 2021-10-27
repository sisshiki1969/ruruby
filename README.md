# ruruby ![ruruby](https://github.com/sisshiki1969/ruruby/workflows/Rust/badge.svg)![codecov](https://codecov.io/gh/sisshiki1969/ruruby/branch/master/graph/badge.svg)

An alternative Ruby implementation by Rust.

## Features

- Purely implemented with Rust.
- No dependency on any other Ruby implementation such as CRuby(MRI), mruby, .. etc.
- Hand-written original parser.
- Virtual machine execution.
- Simple mark & sweep garbage collector is implemented.
- Supporting x86/posix, arm64/macos, x86/windows (thanks @jtran and @dmtaub!) . 64-bit arch only.

## Related articles (sorry, currently only in Japanese)

[Qiita: Rust でつくる（つくれるかもしれない）Ruby (You can (possibly) make Ruby.)](https://qiita.com/sisshiki1969/items/3d25aa81a376eee2e7c2)  
[Qiita: ruruby: Rust でつくっている Ruby (Making Ruby with Rust)](https://qiita.com/sisshiki1969/items/4d76e69545ca1c26ed48)  
[SpeakerDeck: Rust でつくるガーベジコレクタ (Garbage collector written in Rust)](https://speakerdeck.com/sisshiki1969/rustdetukurugabezikorekuta)

## Implementation status

[See Wiki.](https://github.com/sisshiki1969/ruruby/wiki/Implementation-status)

## Optcarrot benchmark

|    benchmark    |       CRuby       |      ruruby       |  rate  |
| :-------------: | :---------------: | :---------------: | :----: |
|    optcarrot    | 56.09 ± 0.13 fps  | 34.80 ± 0.08 fps  | x 1.61 |
| optcarrot --opt | 130.53 ± 0.82 fps | 101.85 ± 1.08 fps | x 1.28 |

<br/>

To check other benchmark results, [see Wiki.](https://github.com/sisshiki1969/ruruby/wiki/Benchmarks)

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

There are some useful options for analysis and development. Use `features` flag of cargo.

### `trace` option: bytecode trace execution

```sh
% cargo run --features trace -- tests/sample.rb
    Finished dev [unoptimized + debuginfo] target(s) in 0.03s
     Running `target/debug/ruruby tests/sample.rb`
+++> MethodId(552) Method[unnamed] "/home/monochrome/ruruby/tests/sample.rb"
--------invoke new context------------------------------------------
Stack context:0x55be3bec7d00 outer:None prev_stack_len:0
  iseq: Ref(0x55be3bedbc90)
  self: #<Object:0x00007f94e2b40140>
  lvar(0): w nil
  delegate: None
--------------------------------------------------------------------
   0: CONST_VAL "world"                        tmp: 0    stack: 0   top:
   5: SET_LOCAL 'w'                            tmp: 0    stack: 1   top: "world"
   a: CONST_VAL "Hello "                       tmp: 0    stack: 0   top:
   f: GET_LOCAL 'w'                            tmp: 0    stack: 1   top: "Hello "
  14: TO_S                                     tmp: 0    stack: 2   top: "world"
  15: CONST_VAL "!"                            tmp: 0    stack: 2   top: "world"
  1a: CONCAT_STR 3 items                       tmp: 0    stack: 3   top: "!"
  1f: O_SEND_SLF 'puts' args:1 block:None      tmp: 0    stack: 1   top: "Hello world!"
+++> BuiltinFunc puts
Hello world!
<+++ Ok(nil)
  2e: RETURN                                   tmp: 0    stack: 1   top: nil
<+++ Ok(nil)
```

### `emit-iseq` option: dump bytecode

```sh
% cargo run --features emit-iseq -- tests/sample.rb
   Compiling ruruby v0.3.1 (/home/monochrome/ruruby)
    Finished dev [unoptimized + debuginfo] target(s) in 9.01s
     Running `target/debug/ruruby tests/sample.rb`
-----------------------------------------
[MethodId(552)] Method: <unnamed> opt:true
local var: 0:w
block: None
  00000 CONST_VAL "world"
  00005 SET_LOCAL 'w'
  0000a CONST_VAL "Hello "
  0000f GET_LOCAL 'w'
  00014 TO_S
  00015 CONST_VAL "!"
  0001a CONCAT_STR 3 items
  0001f O_SEND_SLF 'puts' args:1 block:None
  0002e RETURN
Hello world!
```

### `perf` option: performance analysis per VM instruction

```sh
% cargo run --release --features perf -- tests/app_mandelbrot.rb > /dev/null
    Finished release [optimized] target(s) in 25.75s
     Running `target/release/ruruby tests/app_mandelbrot.rb`
+-------------------------------------------+
| Performance stats for inst:               |
| Inst name         count    %time  ns/inst |
+-------------------------------------------+
  PUSH_VAL          5385K     5.57       37
  PUSH_NIL          3948K     3.97       36
  CONST_VAL             2     0.00      100
  SET_LOCAL          320K     0.39       43
  GET_LOCAL          620K     0.66       38
  SET_DYNLOCAL      4265K     4.69       39
  GET_DYNLOCAL        16M    17.25       36
  O_SEND            3925K     6.16       56
  O_SEND_SLF         480K     0.93       69
  O_SEND_N           160K     0.30       66
  O_SEND_SLF_N        20K     0.04       67
  DUP                 20K     0.03       51
  CONCAT_STR            1     0.00      400
  TO_S                  2     0.00      600
  DEF_METHOD            1     0.00      300
  RETURN            4032K     6.82       60
  MRETURN             96K     0.20       74
  ADD               3905K     6.26       57
  SUB                320K     0.42       47
  MUL               4225K     6.55       55
  DIV                320K     0.41       45
  SHL                160K     0.22       48
  BIT_OR             160K     0.20       44
  ADDI               160K     0.20       44
  SUBI               140K     0.17       44
  JMP_F_EQ           140K     0.19       49
  JMP_F_GT          3905K     4.57       41
  JMP_F_EQI          160K     0.22       49
  GC                  697     0.75    38699
  EXTERN              12M    32.82       96
```

Instruction name, total execution count, percentage in the total execution time, and
execution time per single instruction are shown.  
\* `GC` means garbage collection.  
\*\* `EXTERN` means exectution of native methods.
