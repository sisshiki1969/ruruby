# structure of stack frame

This document describes a structure of stack frames.

## Ruby method/block

         lfp                                             cfp                                                                   sp
          v                                               v                                                                     v
     --+------+------+--+------+------+--+------+------+------+------+------+------+------+------+------+------+------+------+-----
       |  a0  |  a1  |..|  an  |  l0  |..|  ln  | self |precfp|pre_sp| flg  | lfp  | mfp  | dfp  |  pc  | heap | iseq | blok |
     --+------+------+--+------+------+--+------+------+------+------+------+------+------+------+------+------+------+------+-----
        <------------ local frame -------------> <------------------------- control frame --------------------------->

- a0..an: arguments
- l0..ln: local variables
- self: self value
- pre_cfp: cfp of the previous frame (always on the stack)
- pre_sp: sp of the previous frame (always on the stack)
- lfp: local frame pointer (on the stack or heap)
- flg: various infomation of current context.
- mfp: method frame pointer (on the stack or heap)
- dfp: outer frame pointer (on the stack or heap)
- pc: current program counter (on iseq)
- heap: if this frame has been moved to heap, this field points to the heap frame.
- iseq: reference to a bytecode (instruction sequence)
- block: a block which passed to current context by caller frame.

## native method frame

         lfp                            cfp                  sp
          v                              v                    v
     --+------+------+--+------+------+------+------+------+-----
       |  a0  |  a1  |..|  an  | self |precfp|pre_sp| flg  |
     --+------+------+--+------+------+------+------+------+-----
        <---- local frame ----> <----- control frame ----->

- a0..an: arguments
- self: self value
