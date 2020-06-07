# ruruby benchmark results

## environment

Ruby version: 2.6.3  
CPU: Intel(R) Core(TM) i9-9880H CPU @ 2.30GHz  
OS: Mac OS X 10.15.4  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 2.50 ± 0.00 s | 4.71 ± 0.13 s | x 1.88 |
| app_mandelbrot.rb | 3.69 ± 0.01 s | 15.65 ± 0.29 s | x 4.25 |
| fibo.rb | 1.07 ± 0.02 s | 6.21 ± 0.30 s | x 5.82 |
| block.rb | 1.18 ± 0.00 s | 2.62 ± 0.06 s | x 2.22 |
| ao_bench.rb | 30.51 ± 4.54 s | 170.95 ± 20.98 s | x 5.60 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 11.3K | 2.3K | x 0.20 |
| app_mandelbrot.rb | 11.4K | 3.7K | x 0.32 |
| fibo.rb | 11.4K | 2.1K | x 0.18 |
| block.rb | 11.1K | 2.0K | x 0.18 |
| ao_bench.rb | 12.0K | 5.3K | x 0.44 |
