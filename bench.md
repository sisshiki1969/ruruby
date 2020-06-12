# ruruby benchmark results

## environment

Ruby version: 2.6.3  
CPU: Intel(R) Core(TM) i9-9880H CPU @ 2.30GHz  
OS: Mac OS X 10.15.5  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.48 ± 0.10 s | 2.44 ± 0.02 s | x 1.65 |
| app_mandelbrot.rb | 1.93 ± 0.01 s | 8.65 ± 0.17 s | x 4.47 |
| fibo.rb | 0.50 ± 0.01 s | 2.36 ± 0.05 s | x 4.72 |
| block.rb | 0.40 ± 0.01 s | 0.97 ± 0.09 s | x 2.42 |
| ao_bench.rb | 8.14 ± 0.32 s | 36.09 ± 1.10 s | x 4.43 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 11.8K | 2.5K | x 0.21 |
| app_mandelbrot.rb | 11.2K | 160.4K | x 14.29 |
| fibo.rb | 11.2K | 2.1K | x 0.19 |
| block.rb | 11.1K | 2.0K | x 0.18 |
| ao_bench.rb | 11.8K | 119.1K | x 10.12 |
