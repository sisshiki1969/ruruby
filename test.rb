class Vec
    def initialize(x,y)
        @x=x;@y=y
    end
    def +(v)
        Vec.new(@x + v.x, @y + v.y)
    end
    def *(v)
        Vec.new(@x * v.x, @y * v.y)
    end
    def x; @x; end
    def y; @y; end
end

v1 = Vec.new(2,4)
v2 = Vec.new(3,5)
v = v1 + v2;
puts(v.x, v.y)
v = v1 * v2;
puts(v.x, v.y)