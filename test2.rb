class Enumerator_
    def initialize(obj, method = :each, *args)
        #p obj, method, args
        @obj = obj.send(method, *args)
        @size = @obj.size
        @f = Fiber.new { ||
            @obj.each { |elem|
                Fiber.yield elem
            }
        }
    end
    def map(&block)
        res = []
        @size.times {
            res << block.call(@f.resume)
        }
        res
    end
    attr_reader :size
end

str = "Yet Another Ruby Hacker"
enum = Enumerator_.new(str, :scan, /\w+/)
upcase = Proc.new {|x| x.upcase}
p enum.map(&upcase)

ary = str.scan(/\w+/)
enum = Enumerator_.new(ary)
p enum.map(&upcase)