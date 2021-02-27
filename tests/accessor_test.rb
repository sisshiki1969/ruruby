class C
    attr_accessor :a, :b
    def initialize
        @a = 100
        @b = 200
    end
end
class D < C
    attr_accessor :c
    def initialize
        @c = 300
        super
    end
end
c = C.new
assert 100, c.a
assert 200, c.b
d = D.new
assert 100, d.a
assert 200, d.b
assert 300, d.c