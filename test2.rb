class Enumerator_
    def initialize(obj, method, *args)
        @obj = obj.send(method, *args)
        @count = @obj.length
        @f = Fiber.new { ||
            @obj.each { |elem|
                Fiber.yield elem
            }
        }
    end
    def map(&block)
        res = []
        @count.times {
            res << block.call(@f.resume)
        }
        res
    end
end

str = "Yet Another Ruby Hacker"
enum = Enumerator_.new(str, :scan, /\w+/)
#upcase = Proc.new {|x| x.upcase}
p enum.map(&:upcase)