# ruruby benchmark results

## environment

Ruby version: 2.7.1  
CPU: Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.83 ± 0.01 s | 2.93 ± 0.03 s | x 1.60 |
| app_mandelbrot.rb | 2.25 ± 0.02 s | 6.70 ± 0.13 s | x 2.98 |
| fibo.rb | 0.50 ± 0.02 s | 2.81 ± 0.02 s | x 5.67 |
| block.rb | 0.38 ± 0.01 s | 1.37 ± 0.02 s | x 3.63 |
| ao_bench.rb | 9.87 ± 0.07 s | 29.06 ± 0.12 s | x 2.94 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 14.2K | 8.9K | x 0.62 |
| app_mandelbrot.rb | 14.2K | 5.1K | x 0.36 |
| fibo.rb | 14.0K | 4.6K | x 0.33 |
| block.rb | 14.0K | 4.5K | x 0.32 |
| ao_bench.rb | 14.7K | 8.4K | x 0.57 |
