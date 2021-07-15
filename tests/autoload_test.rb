class Foo
  autoload :Bar, File.join(__dir__, 'autoload_sample.rb')
end
#p Foo::Bar
assert 100, Foo::Bar