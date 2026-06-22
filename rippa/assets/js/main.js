/**
 * Shows or hides the extra hidden content
 * @param {string} id The calling element's ID
 * @param {string} contentId The ID of the hidden element
 */
function toggleHiddenContent(id, contentId) {
    let self = document.getElementById(id);
    let content = document.getElementById(contentId);

    self.classList.toggle("active");
    if (content.style.maxHeight) {
        content.style.maxHeight = null;
        self.setAttribute("src", "/assets/svg/chevron-right.svg");
    } else {
        content.style.maxHeight = content.scrollHeight + 10 + "px";
        self.setAttribute("src", "/assets/svg/chevron-down.svg");
    }
}
