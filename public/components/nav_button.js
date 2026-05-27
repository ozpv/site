const button = document.getElementById("link-button");
const links = document.getElementById("link-list");
const open = document.getElementById("link-button-open");
const close = document.getElementById("link-button-close");

button.addEventListener("click", () => {
	if (links.classList.contains("rm")) {
		links.classList.remove("rm");
		
		open.classList.add("rm");
		close.classList.remove("rm");
	} else {
		links.classList.add("rm");
		
		open.classList.remove("rm");
		close.classList.add("rm");
	}
});