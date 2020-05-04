# ruruby ![ruruby](https://github.com/sisshiki1969/ruruby/workflows/Rust/badge.svg)

An alternative Ruby implementation by Rust.

## Feature

- Purely implemented with Rust. No dependency on any other Ruby implementation such as CRuby(MRI), mruby, .. etc.
- Hand-written original parser.
- Virtual machine execution.

## Related article

[Qiita: Rust でつくる（つくれるかもしれない）Ruby](https://qiita.com/sisshiki1969/items/3d25aa81a376eee2e7c2)

## Implementation status

- Literals
  - [x] Bool
  - [x] Integer
  - [x] Float
  - [x] String literal
  - [x] String literal with interpolation
  - [x] Array literal
  - [x] Hash literal
  - [x] Lambda literal
  - [x] Block literal
  - [x] Regular expression
- Objects
  - [x] Integer
  - [x] Float
  - [x] Symbol
  - [x] String
  - [x] Range
  - [x] Array
  - [x] Hash
  - [x] Proc
  - [x] Method
  - [x] Regexp
  - [x] Struct
  - [x] Enumerator
  - [x] Fiber
- Variables
  - [x] Local variable
  - [x] Instance variable
  - [ ] Class variable
  - [x] Global variable
- Constants
  - [x] Constant
- Branch and Loop
  - [x] If-then-elsif-else
  - [x] Unless-then-else
  - [x] Postfix if / unless
  - [x] For-in
  - [x] Break / Continue
  - [x] While
  - [x] Until
  - [x] Postfix while / until
  - [x] Case-when
  - [x] Return
- Methods
  - [x] Instance Method
  - [x] Class Method
  - [x] Singleton Method
- Class and Module
  - [x] Subclass / Inheritance
  - [x] Initializer
  - [x] Attribute accessor
  - [x] Open class (Ad-hoc class definition)
  - [x] Module

## How to run ruruby

To build ruruby, You'll need installation of Rust.
Be aware that only nightly version of Rust works for ruruby.

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

### Option: Bytecode Trace execution

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

### Option: Emit ByteCode

```sh
% cargo run --features emit-iseq -- tests/sample.rb
   Compiling ruruby v0.1.0
    Finished dev [unoptimized + debuginfo] target(s) in 6.72s
     Running `target/debug/ruruby tests/sample.rb`
-----------------------------------------
MethodRef(198)
local var: 0:w
block: None
  00000 PUSH_STRING 181
  00005 SET_LOCAL outer:0 LvarId:0
  00014 PUSH_STRING 182
  00019 PUSH_STRING 183
  00024 CONCAT_STR
  00025 GET_LOCAL outer:0 LvarId:0
  00034 TO_S
  00035 CONCAT_STR
  00036 PUSH_STRING 184
  00041 CONCAT_STR
  00042 SEND_SELF 'puts' 1 items
  00059 END
Hello world!
```

### Option: Performance analysis per VM instruction

```sh
% cargo run --features perf -- tests/sample.rb
   Compiling ruruby v0.1.0
    Finished dev [unoptimized + debuginfo] target(s) in 3.53s
     Running `target/debug/ruruby tests/sample.rb`
Hello world!
Performance analysis for Inst:
------------------------------------------
Inst name         count    %time     nsec
                                    /inst
------------------------------------------
PUSH_STRING           4     6.48    25993
SET_LOCAL             1     0.19     3756
GET_LOCAL             1     0.00      382
SEND_SELF             1     2.58    41261
CONCAT_STR            3     0.19     1166
TO_S                  1     0.25     4707
END                   1     0.06     1044
CODEGEN               1    87.48  1391588
EXTERN                1     2.45    39834
------------------------------------------
```
