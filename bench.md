# ruruby benchmark results

## environment

Ruby version: 2.6.3  
CPU: Intel(R) Core(TM) i9-9880H CPU @ 2.30GHz  
OS: Mac OS X 10.15.4  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.37 ± 0.02 s | 2.85 ± 0.09 s | x 2.09 |
| app_mandelbrot.rb | 2.01 ± 0.01 s | 8.56 ± 0.14 s | x 4.26 |
| fibo.rb | 0.49 ± 0.01 s | 2.64 ± 0.13 s | x 5.37 |
| block.rb | 0.43 ± 0.01 s | 0.90 ± 0.01 s | x 2.10 |
| ao_bench.rb | 8.04 ± 0.24 s | 36.29 ± 0.46 s | x 4.51 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 11.4K | 2.3K | x 0.20 |
| app_mandelbrot.rb | 11.0K | 3.7K | x 0.34 |
| fibo.rb | 11.1K | 2.1K | x 0.19 |
| block.rb | 11.3K | 2.0K | x 0.18 |
| ao_bench.rb | 11.7K | 9.1K | x 0.78 |
