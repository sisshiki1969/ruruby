class Enumerator
    def initialize(receiver, method = :each, *args)
    @receiver = receiver
    @method = method
    @fiber = Fiber.new do
      receiver.send(method, *args) do |x|
        Fiber.yield(x)
      end
      raise StopIteration
    end
  end
  def inspect
    "Enum #{@receiver} #{@method}"
  end
  def next
    @fiber.resume
  end
  def each
    if block_given?
      loop do
        yield @fiber.resume
      end
    end
    self
  end
  def map(*args)
    if block_given?
      a = []
      loop do
        a << yield(@fiber.resume)
      end
      a
    else
      Enumerator.new(self, :map, *args)
    end
  end
  def with_index(*args)
    if block_given?
      c = 0
      loop do
        yield(@fiber.resume, c)
        c += 1
      end
      @receiver
    else
      Enumerator.new(self, :with_index, *args)
    end
  end
end
