module Foo
    def bow
        p "bow"
    end
end
class Bar
  include Foo
end
class Baz < Bar
  p ancestors
  p included_modules
  p superclass
end
Baz.new.bow