module M2
end

module M1
  include M2
end

class S
end

class C < S
  include M1
end

p C.ancestors[0] == C
p C.ancestors[1] == M1
p C.ancestors[2] == M2
p C.ancestors[3] == S