class Enumerator
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
  def with_index(*args)
    if block_given?
      c = 0
      a = []
      loop do
        a << yield(@fiber.resume, c)
        c += 1
      end
      a
    else
      Enumerator.new(self, :with_index, *args)
    end
  end
end
