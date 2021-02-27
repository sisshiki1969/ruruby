class C
  attr_accessor :a
end

o = C.new
i = 0
while i < 2000000 do
    a = o.a
    a = o.a
    a = o.a
    a = o.a
    a = o.a
    a = o.a
    a = o.a
    a = o.a
    a = o.a
    a = o.a
    a = o.a
    a = o.a
    a = o.a
    a = o.a
    a = o.a
    a = o.a
    a = o.a
    a = o.a
    a = o.a
    a = o.a
    i += 1
end
