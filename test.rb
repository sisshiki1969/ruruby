class Pos
  def initialize
    @x = [nil] * 10
  end
end

class Vec
  def initialize
    @x = Pos.new
    @y = Pos.new
  end
end
a = []
5.times.each { |x|
  a << Vec.new
}
p a