# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|     benchmark     |     ruby      |     ruruby     |  rate  |
| :---------------: | :-----------: | :------------: | :----: |
| so_mandelbrot.rb  | 1.92 ± 0.01 s | 3.24 ± 0.02 s  | x 1.69 |
| app_mandelbrot.rb | 2.33 ± 0.02 s | 6.38 ± 0.05 s  | x 2.74 |
|      fibo.rb      | 0.49 ± 0.00 s | 2.13 ± 0.02 s  | x 4.39 |
|     block.rb      | 0.39 ± 0.01 s | 1.16 ± 0.05 s  | x 2.95 |
|    ao_bench.rb    | 9.93 ± 0.08 s | 27.71 ± 0.15 s | x 2.79 |
## optcarrot benchmark

|    benchmark    |       ruby        |      ruruby      |  rate  |
| :-------------: | :---------------: | :--------------: | :----: |
|    optcarrot    | 45.79 ± 1.96 fps  | 7.77 ± 0.12 fps  | x 5.90 |
| optcarrot --opt | 135.48 ± 0.74 fps | 33.44 ± 3.91 fps | x 4.05 |

## memory consumption

|     benchmark     | ruby  | ruruby |  rate   |
| :---------------: | :---: | :----: | :-----: |
| so_mandelbrot.rb  | 14.0K |  4.8K  | x 0.34  |
| app_mandelbrot.rb | 14.0K | 17.4K  | x 1.24  |
|      fibo.rb      | 13.8K |  4.7K  | x 0.34  |
|     block.rb      | 13.8K |  4.6K  | x 0.33  |
|    ao_bench.rb    | 14.5K | 20.5K  | x 1.42  |
|     optcarrot     | 68.4K | 91.2K  | x 1.33  |
|  optcarrot --opt  | 0.1M  |  4.6M  | x 53.05 |
