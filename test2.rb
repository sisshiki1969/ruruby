class Foo
    attr_accessor :a
    def initialize
        @a = 0
    end
    def inc
        @a = @a + 1
        self
    end
end

x = Foo
.new
.inc
.inc
.a

puts x
