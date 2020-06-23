# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|     benchmark     |     ruby      |     ruruby     |  rate  |
| :---------------: | :-----------: | :------------: | :----: |
| so_mandelbrot.rb  | 1.75 ± 0.03 s | 2.80 ± 0.02 s  | x 1.60 |
| app_mandelbrot.rb | 2.18 ± 0.02 s | 5.90 ± 0.04 s  | x 2.70 |
|      fibo.rb      | 0.48 ± 0.05 s | 1.89 ± 0.02 s  | x 3.91 |
|     block.rb      | 0.37 ± 0.00 s | 0.96 ± 0.01 s  | x 2.63 |
|    ao_bench.rb    | 9.35 ± 0.17 s | 26.78 ± 0.17 s | x 2.87 |

## optcarrot benchmark

|    benchmark    |       ruby        |      ruruby      |  rate  |
| :-------------: | :---------------: | :--------------: | :----: |
|    optcarrot    | 47.00 ± 0.23 fps  | 8.04 ± 0.44 fps  | x 5.84 |
| optcarrot --opt | 139.95 ± 3.36 fps | 35.97 ± 0.35 fps | x 3.89 |

## memory consumption

|     benchmark     | ruby  | ruruby |  rate   |
| :---------------: | :---: | :----: | :-----: |
| so_mandelbrot.rb  | 14.1K |  4.7K  | x 0.33  |
| app_mandelbrot.rb | 14.0K |  6.4K  | x 0.45  |
|      fibo.rb      | 13.8K |  4.6K  | x 0.34  |
|     block.rb      | 13.8K |  4.6K  | x 0.33  |
|    ao_bench.rb    | 14.5K |  7.2K  | x 0.50  |
|     optcarrot     | 68.4K | 81.3K  | x 1.19  |
|  optcarrot --opt  | 0.1M  |  1.5M  | x 17.83 |
