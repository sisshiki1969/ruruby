# structure of stack frame

This document describes a structure of stack frames.

## Ruby method/block

      pre_sp        lfp                                             ep                                                       cfp          sp
        v            v                                               v                                                        v            v
     ------+------+------+------+--+------+------+--+------+------+------+------+------+------+------+------+------+------+------+------+-----
           | self |  a0  |  a1  |..|  an  |  l0  |..|  ln  | self | lfp  | mfp  |outer |  pc  |  ep  | iseq | blok |precfp|pre_sp| flg  |
     ------+------+------+------+--+------+------+--+------+------+------+------+------+------+------+------+------+------+------+------+-----
                   <------------ local frame -------------> <----------------- environment frame -----------------> <cont. frame>

- a0..an: arguments
- l0..ln: local variables
- self: self value
- pre_cfp: cfp of the previous frame (always on the stack)
- pre_sp: sp of the previous frame (always on the stack)
- lfp: local frame pointer (on the stack or heap)
- flg: various infomation of current context.
- mfp: method frame pointer (on the stack or heap)
- outer: outer frame pointer (on the stack or heap)
- pc: current program counter (on iseq)
- ep: current environment frame pointer. if this frame has been moved to heap, this field points to the heap frame.
- iseq: reference to a bytecode (instruction sequence)
- block: a block which passed to current context by caller frame.

## native method frame

     pre_sp        lfp                            cfp                  sp
       v            v                              v                    v
    ------+------+------+------+--+------+------+------+------+------+-----
          | self |  a0  |  a1  |..|  an  | self |precfp|pre_sp| flg  |
    ------+------+------+------+--+------+------+------+------+------+-----
                  <---- local frame ----> <----- control frame ----->

- a0..an: arguments
- self: self value
