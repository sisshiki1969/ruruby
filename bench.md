# ruruby benchmark results

## environment

Ruby version: 2.6.3  
CPU: Intel(R) Core(TM) i9-9880H CPU @ 2.30GHz  
OS: Mac OS X 10.15.4  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.30 ± 0.02 s | 2.46 ± 0.01 s | x 1.89 |
| app_mandelbrot.rb | 1.96 ± 0.03 s | 8.41 ± 0.23 s | x 4.30 |
| fibo.rb | 0.51 ± 0.01 s | 2.73 ± 0.03 s | x 5.40 |
| block.rb | 0.44 ± 0.00 s | 0.97 ± 0.02 s | x 2.19 |
| ao_bench.rb | 7.93 ± 0.11 s | 35.40 ± 0.57 s | x 4.46 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 11.4K | 2.3K | x 0.20 |
| app_mandelbrot.rb | 11.2K | 3.5K | x 0.32 |
| fibo.rb | 11.5K | 2.1K | x 0.18 |
| block.rb | 11.1K | 2.0K | x 0.18 |
| ao_bench.rb | 11.7K | 9.1K | x 0.78 |
