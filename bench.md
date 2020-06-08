# ruruby benchmark results

## environment

Ruby version: 2.6.3  
CPU: Intel(R) Core(TM) i9-9880H CPU @ 2.30GHz  
OS: Mac OS X 10.15.4  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.45 ± 0.09 s | 2.75 ± 0.06 s | x 1.90 |
| app_mandelbrot.rb | 2.05 ± 0.04 s | 9.19 ± 0.15 s | x 4.49 |
| fibo.rb | 0.52 ± 0.00 s | 2.68 ± 0.08 s | x 5.20 |
| block.rb | 0.51 ± 0.02 s | 1.07 ± 0.02 s | x 2.11 |
| ao_bench.rb | 9.01 ± 0.17 s | 38.79 ± 1.33 s | x 4.31 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 11.7K | 2.5K | x 0.21 |
| app_mandelbrot.rb | 11.8K | 6.0K | x 0.51 |
| fibo.rb | 11.6K | 2.1K | x 0.18 |
| block.rb | 11.9K | 2.0K | x 0.17 |
| ao_bench.rb | 11.9K | 12.0K | x 1.00 |
