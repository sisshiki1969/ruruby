#!/usr/local/bin/ruby
# frozen_string_literal: true

require 'open3'

if RUBY_PLATFORM =~ /linux/
  @platform = :linux
elsif RUBY_PLATFORM =~ /(darwin|mac os)/
  @platform = :macos
elsif RUBY_PLATFORM =~ /mswin(?!ce)|mingw|cygwin|bccwin/
  @platform = :windows
else
  raise 'unknown platform'
end

`ruby -v`.match(/ruby (\d*).(\d*).(\d*)/) do
  @ruby_version = "#{Regexp.last_match(1)}.#{Regexp.last_match(2)}.#{Regexp.last_match(3)}"
end

if @platform == :macos
  `sysctl machdep.cpu.brand_string`.match(/brand_string:\s*(.*)/)
elsif @platform == :windows
  @win = `systeminfo`.encode(Encoding::UTF_8, Encoding::SHIFT_JIS)
  @win.match(/プロセッサ.*\r\n(.*)/)
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
elsif @platform == :windows
  @win.match(/OS 名：\s*(.*)/)
  @os_info = Regexp.last_match(1)
else
  `cat /etc/os-release`.match(/PRETTY_NAME=\"(.*)\"/)
  @os_info = Regexp.last_match(1)
end
puts @os_info

@time_command = if @platform == :macos
                  'gtime'
                else
                  '/usr/bin/time'
                end
@branch = `git branch --show-current`.chomp

@time = Time.now

puts @time
puts "Ruby version: #{@ruby_version}"
puts "CPU: #{@cpu_info}"
puts "OS: #{@os_info}"
puts "branch: #{@branch}"

if @platform != :windows
  `set -x`
end
`cargo build --release`

@ruruby_exec = File.expand_path('../target/release/ruruby', __dir__)
@optcarrot_dir = File.expand_path('../../optcarrot', __dir__)

def unit_conv(ruruby, ruby)
  ch = ''
  if ruruby > 1_000_000.0
    ruruby /= 1_000_000.0
    ruby /= 1_000_000.0
    ch = 'M'
  elsif ruruby > 1000
    ruruby /= 1000.0
    ruby /= 1000.0
    ch = 'K'
  end
  [ruruby, ruby, ch]
end

def print_cmp(kind, ruby, ruruby, ratio)
  ruruby, ruby, ch = unit_conv(ruruby, ruby)

  if ruby.is_a?(Float)
    res_ruby = "%6.2f#{ch}" % ruby
    res_ruruby = format("%6.2f#{ch}", ruruby)
  else
    res_ruby = "%6d#{ch}" % ruby
    res_ruruby = format("%6d#{ch}", ruruby)
  end
  puts format("#{kind}\t%10s  %10s  x %7.2f", res_ruby, res_ruruby, ratio)
end

class Array
  def ave_sd
    ave = sum.to_f / length
    sd = Math.sqrt(map { |x| (x - ave) ** 2 }.sum / length)
    { ave: ave, sd: sd }
  end
end

def print_avesd(ave_sd)
  "#{'%.2f' % ave_sd[:ave]} ± #{'%.2f' % ave_sd[:sd]}"
end

def optcarrot(program, option = "")
  command = "#{@time_command} #{program} #{@optcarrot} #{option}"
  fps = []
  rss = []
  5.times do
    o, e, s = Open3.capture3(command)
    o.match(/fps: (\d*.\d*)/)
    fps << Regexp.last_match(1).to_f
    o.match(/checksum: (\d*)/)
    checksum = Regexp.last_match(1).to_i
    if checksum == 59662
      puts "optcarrot checksum ok:"
    else
      puts "checksum invalid: #{checksum}"
    end
    e.match(/(\d*)maxresident/)
    rss << Regexp.last_match(1).to_i * 1000
  end
  [fps.ave_sd, rss.ave_sd]
end

def perf_optcarrot(option = "")
  fps_ruby, rss_ruby = optcarrot('ruby', option)
  fps_ruruby, rss_ruruby = optcarrot(@ruruby_exec, option)

  puts "benchmark: optcarrot #{option}"
  puts format("\t%10s  %10s", 'ruby', 'ruruby')
  print_cmp('fps', fps_ruby[:ave], fps_ruruby[:ave], fps_ruby[:ave]/fps_ruruby[:ave])
  print_cmp('rss', rss_ruby[:ave], rss_ruruby[:ave], rss_ruruby[:ave]/rss_ruby[:ave])

  rss_mul = rss_ruruby[:ave] / rss_ruby[:ave]
  rss_ruruby, rss_ruby, ch = unit_conv(rss_ruruby[:ave], rss_ruby[:ave])
end

@optcarrot = [@optcarrot_dir + "/bin/optcarrot", "-b", @optcarrot_dir + "/examples/Lan_Master.nes"].join(" ")

["", "--opt"].each do |x|
  perf_optcarrot(x)
end

