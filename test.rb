require 'json'
h = JSON.load(<<J, symbolize_names: true)
{"name":"ko1","age":17}
J
