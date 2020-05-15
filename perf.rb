#!/usr/local/bin/ruby
# frozen_string_literal: true

require 'open3'

@md1 = "# ruruby benchmark results\n\n|benchmark|ruby|ruruby|rate|\n|:-----------:|:--------:|:---------:|:-------:|\n"

`set -x`
`cargo build --release`

def print_cmp(kind, ruby, ruruby)
  ch = ''
  if ruruby > 1_000_000
    ruruby = ruruby.to_f / 1_000_000
    ruby = ruby.to_f / 1_000_000
    ch = ' M'
  elsif ruruby > 1000
    ruruby = ruruby.to_f / 1000
    ruby = ruby.to_f / 1000
    ch = ' K'
  end
  if ruby.is_a?(Float)
    res_ruby = "%6.2f#{ch}" % ruby
    res_ruruby = format("%6.2f#{ch}", ruruby)
  else
    res_ruby = "%6d#{ch}" % ruby
    res_ruruby = format("%6d#{ch}", ruruby)
  end
  puts format("#{kind}\t%10s  %10s  x %7.2f", res_ruby, res_ruruby, ruruby.to_f / ruby.to_f)
end

def get_results(e)
  e.match(/real\s*(\d*).(\d*)/)
  real = "#{Regexp.last_match(1)}.#{Regexp.last_match(2)}".to_f
  e.match(/user\s*(\d*).(\d*)/)
  user = "#{Regexp.last_match(1)}.#{Regexp.last_match(2)}".to_f
  e.match(/sys\s*(\d*).(\d*)/)
  sys = "#{Regexp.last_match(1)}.#{Regexp.last_match(2)}".to_f

  e.match(/(\d*)\s*maximum resident set size/)
  rss = Regexp.last_match(1).to_i

  [real, user, sys, rss]
end

def perf(app_name)
  puts "benchmark: #{app_name}"
  o, e, s = Open3.capture3("/usr/bin/time -lp ruby tests/#{app_name} > /dev/null")
  real_ruby, user_ruby, sys_ruby, rss_ruby = get_results(e)

  o, e, s = Open3.capture3("/usr/bin/time -lp ./target/release/ruruby tests/#{app_name} > mandel.ppm")
  real_ruruby, user_ruruby, sys_ruruby, rss_ruruby = get_results(e)

  # `convert mandel.ppm mandel.jpg`
  puts format("\t%10s  %10s", 'ruby', 'ruruby')
  print_cmp('real', real_ruby, real_ruruby)
  print_cmp('user', user_ruby, user_ruruby)
  print_cmp('sys', sys_ruby, sys_ruruby)
  print_cmp('rss', rss_ruby, rss_ruruby)
  mul = real_ruruby.to_f / real_ruby.to_f
  @md1 += "| #{app_name} | #{real_ruby} s | #{real_ruruby} s | x#{'%.2f' % mul} |\n"
end

['so_mandelbrot.rb',
 'app_mandelbrot.rb',
 'fibo.rb',
 'block.rb',
 'ao_bench.rb'].each { |x| perf x }

File.open('perf.md', mode = 'w') do |f|
  f.write(@md1) # ファイルに書き込む
end
