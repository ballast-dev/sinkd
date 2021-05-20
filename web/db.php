<?php
session_start();
$page = $_POST['page'];
$link = new mysqli('localhost', 'tony', 'g3tsink3d', 'sinkd');

//check connection
if ($link->connect_error) {
	die("Connection failed: " . $link->connect_error);
}

//:::::::::::::::::::::::::::
//:::::: logging in::::::::::
//:::::::::::::::::::::::::::

if ($page == "login") {
	// check to see if user exists first
	$username = $link->escape_string($_POST['username']);
	$qresult = $link->query("SELECT * FROM login WHERE username='$username'");

	//now take result and fetch pass out of it
	$user_row = $qresult->fetch_assoc();

	$pass = $_POST['password'];
	$dbpass = $user_row['password'];

	if ($qresult->num_rows == 0) { //user not in database!
		$message = "You are not in the database!";
		echo "<script type='text/javascript'>alert('$message');</script>";

	} elseif (password_verify($pass, $dbpass)){
		$message = "Correct Password!";
		echo "<script type='text/javascript'>alert('$message');</script>";
		$_SESSION['login'] = true;
		$_SESSION['username'] = $user_row['username'];
		$_SESSION['user_path'] = $user_row['filepath'];
		$_SESSION['displayname'] = $user_row['displayname'];
		header("location: main.php");
	}

	else {
		$message = "Wrong password! pass-> $pass, dbpass-> $dbpass";
		echo "<script type='text/javascript'>alert('$message');</script>";
	}
	$link->close();

//::::::::::::::::::::::::::::::::
//::::::::: signing up :::::::::::
//::::::::::::::::::::::::::::::::

} else if ($page == "signup"){

	$pass = $_POST['password'];
	$cpass = $_POST['confirmpassword'];

	//check to see if user exists
	$qresult = $link->query("SELECT * FROM login WHERE username='$username'");



	if ($pass == $cpass){

		$displayname = $link->escape_string($_POST['displayname']);
		$username = $link->escape_string($_POST['username']);
		$password = $link->escape_string($_POST['password']);
		$hash = password_hash($password, PASSWORD_DEFAULT);
		$user_path = $username . "/";

			//need to create user
		$insertsql = "INSERT INTO login (username, password, displayname) VALUES ('$username','$hash','$displayname')";

		if ($link->query($insertsql)) {
			$message = "SUCESSFUL INSERT";
			echo "<script type='text/javascript'>alert('$message');</script>";

			$root_dir = $_SERVER['DOCUMENT_ROOT'] . '/db/' . $user_path;

			if(!file_exists($root)) {
				$message = "Directory = $root_dir";
				echo "<script type='text/javascript'>alert('$message');</script>";

				mkdir($root_dir, 0777);
				$_SESSION['login'] = true;
				$_SESSION['username'] = $username;
				$_SESSION['user_path'] = $user_path;
				$_SESSION['displayname'] = $displayname;

				header("location: main.php");
			} else {
				$message = "Directory already present";
				echo "<script type='text/javascript'>alert('$message');</script>";
				die();
			}
			$message = "Already in database";
			echo "<script type='text/javascript'>alert('$message');</script>";
		}

	} else { //passwords don;t match
		$message = "Passwords don't match";
		echo "<script type='text/javascript'>alert('$message');</script>";
	}
}
?>
