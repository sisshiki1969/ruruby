# ruruby benchmark results

## environment

Ruby version: 2.7.1  
CPU: Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|     benchmark     |       ruby       |     ruruby      |  rate  |
| :---------------: | :--------------: | :-------------: | :----: |
| so_mandelbrot.rb  |  1.88 ± 0.01 s   |  2.71 ± 0.06 s  | x 1.44 |
| app_mandelbrot.rb |  2.29 ± 0.05 s   |  6.23 ± 0.13 s  | x 2.72 |
|      fibo.rb      |  0.48 ± 0.03 s   |  1.99 ± 0.02 s  | x 4.12 |
|     block.rb      |  0.38 ± 0.00 s   |  1.07 ± 0.01 s  | x 2.79 |
|    ao_bench.rb    |  10.36 ± 0.23 s  | 26.33 ± 0.45 s  | x 2.54 |
|     optcarrot     | 45.86 ± 2.57 fps | 7.97 ± 0.17 fps | x 5.76 |

## memory consumption

|     benchmark     | ruby  | ruruby |  rate  |
| :---------------: | :---: | :----: | :----: |
| so_mandelbrot.rb  | 14.3K |  4.8K  | x 0.34 |
| app_mandelbrot.rb | 14.2K | 17.3K  | x 1.22 |
|      fibo.rb      | 14.0K |  4.6K  | x 0.33 |
|     block.rb      | 14.0K |  4.6K  | x 0.33 |
|    ao_bench.rb    | 14.7K | 21.8K  | x 1.49 |
|     optcarrot     | 70.5K | 88.5K  | x 1.25 |
