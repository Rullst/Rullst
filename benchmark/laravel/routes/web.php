<?php

use Illuminate\Support\Facades\Route;
use App\Models\World;

Route::get('/text', function () {
    return 'Hello World';
});

Route::get('/json', function () {
    return response()->json(['message' => 'Hello World']);
});

Route::get('/db-single', function () {
    // Basic query using DB facade assuming a 'world' table exists.
    $world = World::find(1);
    return response()->json($world);
});

Route::get('/html', function () {
    return view('welcome', ['message' => 'Hello World']);
});
