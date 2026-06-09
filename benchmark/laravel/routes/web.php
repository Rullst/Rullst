<?php

use Illuminate\Support\Facades\Route;

Route::get('/', function () {
    return response('Hello, World!', 200)
        ->header('Content-Type', 'text/plain');
});

Route::get('/json', function () {
    return response()->json(['message' => 'Hello, World!']);
});
