# structure of stack frame

         lfp                                             cfp                                                            sp
          v                                               v                                                              v
     --+------+------+--+------+------+--+------+------+------+------+------+------+------+------+------+------+------+-----
       |  a0  |  a1  |..|  an  |  l0  |..|  ln  | self | flag |precfp| mfp  | dfp  |  pc  | ctx  | iseq | lfp  | blok |
     --+------+------+--+------+------+--+------+------+------+------+------+------+------+------+------+------+------+-----
        <------------ local frame -------------> <------------------------- control frame --------------------------->

- a0..an: arguments
- l0..ln: local variables
- self: self value
- flag: various infomation of current context.
- precfp: cfp of previous frame (always on the stack)
- mfp: method frame pointer (on the stack or heap)
- dfp: outer frame pointer (on the stack or heap)
- pc: current program counter (on iseq)
- iseq: reference to a bytecode (instruction sequence)
- lfp: local frame pointer (on the stack or heap)
- block: a block which passed to current context by caller frame.
