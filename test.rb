class Foo
    def init
        def self.single
            puts 77
        end
    end
    def single
        puts 99
    end
end

f = Foo.new
f.single
f.init
f.single
class Foo
    def single
        puts 200
    end
end
f.single
Foo.new.single