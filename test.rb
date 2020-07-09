class Enum
  def initialize(receiver, method = :each, *args)
    @fiber = Fiber.new do
      receiver.send(method, *args) do |x|
        Fiber.yield(x)
      end
      raise StopIteration
    end
  end
  def next
    @fiber.resume
  end
  def each
    if block_given?
      loop do
        yield @fiber.resume
      end
    else
      loop do
        @fiber.resume
      end
    end
  end
end

str = "Yet Another Ruby Hacker"
e = Enum.new(str, :scan, /\w+/)
res = []
e.each { |x| res << x }
p res