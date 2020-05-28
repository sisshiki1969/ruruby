#!/usr/local/bin/ruby
# frozen_string_literal: true

require 'open3'

if RUBY_PLATFORM =~ /linux/
  @platform = :linux
elsif RUBY_PLATFORM =~ /(darwin|mac os)/
  @platform = :macos
else
  raise 'unknown platform'
end

`ruby -v`.match(/ruby (\d*).(\d*).(\d*)/) { @ruby_version = "#{Regexp.last_match(1)}.#{Regexp.last_match(2)}.#{Regexp.last_match(3)}" }

if @platform == :macos
  `sysctl machdep.cpu.brand_string`.match(/brand_string:\s*(.*)/)
else
  `cat /proc/cpuinfo`.match(/model name\s*:\s(.*)/)
end
@cpu_info = Regexp.last_match(1)

if @platform == :macos
  sw = `sw_vers`
  sw.match(/ProductName:\s*(.*)/)
  @os_info = Regexp.last_match(1)
  sw.match(/ProductVersion:\s*(.*)/)
  @os_info += ' ' + Regexp.last_match(1)
else
  `cat /etc/os-release`.match(/PRETTY_NAME=\"(.*)\"/)
  @os_info = Regexp.last_match(1)
end

@time_command = if @platform == :macos
                  'gtime'
                else
                  '/usr/bin/time'
                end

@md0 = "# ruruby benchmark results\n
## environment\n
Ruby version: #{@ruby_version}  \nCPU: #{@cpu_info}  \nOS: #{@os_info}  \n\n"

puts "Ruby version: #{@ruby_version}"
puts "OS: #{@os_info}"
puts "CPU: #{@cpu_info}"

@md1 = "## execution time\n
|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
"
@md2 = "\n## memory consumption\n
|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
"

`set -x`
`cargo build --release`

def unit_conv(ruruby, ruby)
  ch = ''
  if ruruby > 1_000_000.0
    ruruby = ruruby / 1_000_000.0
    ruby = ruby / 1_000_000.0
    ch = 'M'
  elsif ruruby > 1000
    ruruby = ruruby / 1000.0
    ruby = ruby / 1000.0
    ch = 'K'
  end
  [ruruby, ruby, ch]
end

def print_cmp(kind, ruby, ruruby)
  ruruby, ruby, ch = unit_conv(ruruby, ruby)

  if ruby.is_a?(Float)
    res_ruby = "%6.2f#{ch}" % ruby
    res_ruruby = format("%6.2f#{ch}", ruruby)
  else
    res_ruby = "%6d#{ch}" % ruby
    res_ruruby = format("%6d#{ch}", ruruby)
  end
  puts format("#{kind}\t%10s  %10s  x %7.2f", res_ruby, res_ruruby, ruruby.to_f / ruby)
end

class Array
  def ave_sd
    ave = self.sum.to_f / self.length
    sd = Math.sqrt(self.map {|x| (x - ave) ** 2}.sum / self.length)
    {ave: ave, sd: sd}
  end
end

def get_results(command)
  real = []
  user = []
  sys = []
  rss = []
  5.times {
    o, e, s = Open3.capture3(command)
    e.match(/(\d*):(\d*).(\d*)elapsed/)
    real << "#{Regexp.last_match(2)}.#{Regexp.last_match(3)}".to_f + Regexp.last_match(1).to_i * 60
    e.match(/(\d*).(\d*)user/)
    user << "#{Regexp.last_match(1)}.#{Regexp.last_match(2)}".to_f
    e.match(/(\d*).(\d*)system/)
    sys << "#{Regexp.last_match(1)}.#{Regexp.last_match(2)}".to_f
    e.match(/(\d*)maxresident/)
    rss << Regexp.last_match(1).to_i
  }

  [real.ave_sd, user.ave_sd, sys.ave_sd, rss.ave_sd]
end

def perf(app_name)
  puts "benchmark: #{app_name}"
  command = "#{@time_command} ruby tests/#{app_name} > /dev/null"
  real_ruby, user_ruby, sys_ruby, rss_ruby = get_results(command)

  command = "#{@time_command} ./target/release/ruruby tests/#{app_name} > mandel.ppm"
  real_ruruby, user_ruruby, sys_ruruby, rss_ruruby = get_results(command)

  # `convert mandel.ppm mandel.jpg`
  puts format("\t%10s  %10s", 'ruby', 'ruruby')
  print_cmp('real', real_ruby[:ave], real_ruruby[:ave])
  print_cmp('user', user_ruby[:ave], user_ruruby[:ave])
  print_cmp('sys', sys_ruby[:ave], sys_ruruby[:ave])
  print_cmp('rss', rss_ruby[:ave], rss_ruruby[:ave])

  real_mul = real_ruruby[:ave] / real_ruby[:ave]
  @md1 += "| #{app_name} | #{'%.2f' % real_ruby[:ave]} ± #{'%.2f' % real_ruby[:sd]} s | #{'%.2f' % real_ruruby[:ave]} ± #{'%.2f' % real_ruruby[:sd]} s | x #{'%.2f' % real_mul} |\n"
  rss_mul = rss_ruruby[:ave] / rss_ruby[:ave]
  rss_ruruby, rss_ruby, ch = unit_conv(rss_ruruby[:ave], rss_ruby[:ave])
  @md2 += "| #{app_name} | #{'%.1f' % rss_ruby}#{ch} | #{'%.1f' % rss_ruruby}#{ch} | x #{'%.2f' % rss_mul} |\n"
end

['so_mandelbrot.rb',
 'app_mandelbrot.rb',
 'fibo.rb',
 'block.rb',
 'ao_bench.rb'].each { |x| perf x }

File.open('bench.md', mode = 'w') do |f|
  f.write(@md0 + @md1 + @md2)
end
