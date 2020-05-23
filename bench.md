# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-7700 CPU @ 3.60GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 1.8 s | 3.05 s | x 1.69 |
| app_mandelbrot.rb | 2.31 s | 7.85 s | x 3.40 |
| fibo.rb | 0.49 s | 2.77 s | x 5.65 |
| block.rb | 0.41 s | 1.07 s | x 2.61 |
| ao_bench.rb | 9.29 s | 30.47 s | x 3.28 |

## memory consumption

|benchmark|ruby|ruruby|rate|
|:-----------:|:--------:|:---------:|:-------:|
| so_mandelbrot.rb | 13.88  K | 11.61  K | x 0.84 |
| app_mandelbrot.rb | 0.01  M | 1.76  M | x 125.89 |
| fibo.rb | 13.67  K | 4.58  K | x 0.34 |
| block.rb | 13.74  K | 4.49  K | x 0.33 |
| ao_bench.rb | 0.01  M | 4.50  M | x 311.27 |
